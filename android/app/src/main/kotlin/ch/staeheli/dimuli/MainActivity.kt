package ch.staeheli.dimuli

import android.app.NativeActivity
import android.content.ClipData
import android.content.ContentValues
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ColorSpace
import android.graphics.ImageDecoder
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.provider.MediaStore
import android.util.Log
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import java.io.File
import java.io.IOException
import java.util.concurrent.Executors
import java.util.concurrent.RejectedExecutionException
import kotlin.math.floor
import kotlin.math.min
import kotlin.math.sqrt

/** Thin host for the Rust app (libdimuli.so); all logic lives on the Rust side. */
class MainActivity : NativeActivity() {
    private var backCallback: OnBackInvokedCallback? = null
    @Volatile private var collageOpen = false
    @Volatile private var collageLost = false

    /** Single worker for file copies; never block the UI thread with I/O. */
    private val io = Executors.newSingleThreadExecutor()

    override fun onCreate(savedInstanceState: Bundle?) {
        collageLost = savedInstanceState?.getBoolean(KEY_COLLAGE_OPEN, false) == true
        super.onCreate(savedInstanceState)
        // From API 35 with targetSdk 37 edge-to-edge is enforced; API 33/34 need the opt-in (D-07).
        if (Build.VERSION.SDK_INT < 35) enableEdgeToEdgeLegacy()
        // Copies left over after process death; picks only start from the start screen, where no
        // collage still uses them.
        io.execute {
            File(cacheDir, "picked").deleteRecursively()
            File(cacheDir, "share").deleteRecursively()
        }
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putBoolean(KEY_COLLAGE_OPEN, collageOpen)
    }

    @Suppress("DEPRECATION")
    private fun enableEdgeToEdgeLegacy() = window.setDecorFitsSystemWindows(false)

    override fun onDestroy() {
        backCallback?.let { onBackInvokedDispatcher.unregisterOnBackInvokedCallback(it) }
        backCallback = null
        io.shutdown()
        super.onDestroy()
    }

    // region JNI bridge
    // Rust -> Kotlin (called on the android_main thread; must return immediately)

    /** Rust: AndroidPlatform (android_main), JNI signature ()Z. */
    fun collageLostOnRestart(): Boolean = collageLost

    /** Rust: AndroidPlatform::set_collage_open, JNI signature (Z)V. */
    fun setCollageOpen(open: Boolean) { collageOpen = open }

    /**
     * Rust: `installBackCallback() -> void` (`call_void` in platform/android.rs), called after
     * `slint::android::init` so our callback is registered after Slint's own.
     */
    fun installBackCallback() = runOnUiThread {
        if (backCallback != null) return@runOnUiThread
        val callback = OnBackInvokedCallback { native { nativeBackPressed() } }
        onBackInvokedDispatcher.registerOnBackInvokedCallback(
            OnBackInvokedDispatcher.PRIORITY_DEFAULT,
            callback,
        )
        backCallback = callback
    }

    /**
     * Rust: `pickImages(max: Int) -> void` (`AndroidPlatform::pick_images` in platform/android.rs),
     * JNI signature `(I)V`. Opens the system Photo Picker; the result arrives in [onActivityResult].
     */
    fun pickImages(max: Int) = runOnUiThread {
        try {
            val intent = Intent(MediaStore.ACTION_PICK_IMAGES).setType("image/*")
            val limit = max.coerceAtMost(MediaStore.getPickImagesMaxLimit())
            // A value above the platform limit makes the picker return RESULT_CANCELED, and the
            // extra must be greater than 1 anyway.
            if (limit > 1) intent.putExtra(MediaStore.EXTRA_PICK_IMAGES_MAX, limit)
            startActivityForResult(intent, REQUEST_PICK)
        } catch (e: RuntimeException) {
            // ActivityNotFound, Security and IllegalState all mean: no picker result will come.
            Log.e(TAG, "cannot open the photo picker: ${e.javaClass.simpleName}")
            native { nativePickFailed() }
        }
    }

    /**
     * Rust: `saveToGallery(jpeg: ByteArray) -> void` (`AndroidPlatform::save_to_gallery` in
     * platform/android.rs), JNI signature `([B)V`. Returns at once; the write runs on [io] and the
     * outcome is reported through [nativeSaved]. MediaStore needs no permission for own inserts.
     */
    fun saveToGallery(jpeg: ByteArray) = runOnIo({ native { nativeSaved(false) } }) {
        var uri: Uri? = null
        val ok = try {
            val values = ContentValues().apply {
                put(MediaStore.Images.Media.DISPLAY_NAME, "dimuli-${System.currentTimeMillis() / 1000}.jpg")
                put(MediaStore.Images.Media.MIME_TYPE, "image/jpeg")
                put(MediaStore.Images.Media.RELATIVE_PATH, Environment.DIRECTORY_PICTURES + "/dimuli")
                put(MediaStore.Images.Media.IS_PENDING, 1)
            }
            val collection = MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
            val inserted = contentResolver.insert(collection, values) ?: throw java.io.IOException("no row")
            uri = inserted
            val stream = contentResolver.openOutputStream(inserted) ?: throw java.io.IOException("no stream")
            stream.use { it.write(jpeg) }
            val done = ContentValues().apply { put(MediaStore.Images.Media.IS_PENDING, 0) }
            contentResolver.update(inserted, done, null, null)
            true
        } catch (e: Exception) {
            Log.e(TAG, "save failed: ${e.javaClass.simpleName}")
            // Only the row created by this call; never an existing gallery image.
            uri?.let {
                try {
                    contentResolver.delete(it, null, null)
                } catch (d: Exception) {
                    Log.e(TAG, "cannot remove half-written row: ${d.javaClass.simpleName}")
                }
            }
            false
        }
        // Reported outside the try: a failure while reporting must not delete the finished row.
        native { nativeSaved(ok) }
    }

    /**
     * Rust: `share(jpeg: ByteArray) -> void` (`AndroidPlatform::share` in platform/android.rs), JNI
     * signature `([B)V`. Returns at once; writes `cacheDir/share/dimuli-collage.jpg` on [io], then opens
     * the system chooser on the UI thread. The file stays (the receiver reads it asynchronously) and is
     * overwritten by the next share. The outcome is reported through [nativeShared].
     */
    fun share(jpeg: ByteArray) = runOnIo({ native { nativeShared(false) } }) {
        try {
            val dir = File(cacheDir, "share")
            if (!dir.isDirectory && !dir.mkdirs()) throw java.io.IOException("cannot create share dir")
            File(dir, ShareProvider.FILE_NAME).writeBytes(jpeg)
        } catch (e: Exception) {
            Log.e(TAG, "cannot write the share file: ${e.javaClass.simpleName}")
            native { nativeShared(false) }
            return@runOnIo
        }
        runOnUiThread {
            try {
                val uri = ShareProvider.uri()
                val send = Intent(Intent.ACTION_SEND).apply {
                    type = "image/jpeg"
                    putExtra(Intent.EXTRA_STREAM, uri)
                    clipData = ClipData.newRawUri(null, uri)
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                }
                startActivity(Intent.createChooser(send, null))
                native { nativeShared(true) }
            } catch (e: RuntimeException) {
                // ActivityNotFound, Security and IllegalState: the chooser did not open.
                Log.e(TAG, "cannot open the share sheet: ${e.javaClass.simpleName}")
                native { nativeShared(false) }
            }
        }
    }

    // Kotlin -> Rust (registered by Rust via RegisterNatives)

    /** Rust: `native_saved` in platform/android.rs, JNI signature `(Z)V`. `ok` = the gallery row is complete. */
    private external fun nativeSaved(ok: Boolean)

    /** Rust: `native_shared` in platform/android.rs, JNI signature `(Z)V`. `ok` = the chooser was opened. */
    private external fun nativeShared(ok: Boolean)

    /**
     * Rust: `native_images_picked` in platform/android.rs, JNI signature `([Ljava/lang/String;I)V`.
     * Absolute paths of the cache copies; an empty array means the user cancelled.
     */
    private external fun nativeImagesPicked(paths: Array<String>, failed: Int)

    /** Rust: `native_pick_copying` in platform/android.rs, JNI signature `()V`. The user picked images. */
    private external fun nativePickCopying()

    /** Rust: `native_pick_failed` in platform/android.rs, JNI signature `()V`. */
    private external fun nativePickFailed()

    /** Rust: `native_back_pressed` in platform/android.rs, JNI signature `()V`. */
    private external fun nativeBackPressed()

    /**
     * Runs [task] on [io]. After [onDestroy] the executor rejects work; [onRejected] then reports the
     * failure so the Rust side does not wait forever (and the JNI/UI thread does not throw).
     */
    private fun runOnIo(onRejected: () -> Unit, task: () -> Unit) {
        try {
            io.execute(task)
        } catch (e: RejectedExecutionException) {
            Log.w(TAG, "activity is shutting down, dropping the request", e)
            onRejected()
        }
    }

    /** Natives exist only after android_main registered them; before that the call must not crash. */
    private inline fun native(call: () -> Unit) =
        try {
            call()
        } catch (e: UnsatisfiedLinkError) {
            Log.w(TAG, "native side not ready", e)
        }
    // endregion

    @Suppress("OVERRIDE_DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != REQUEST_PICK) {
            @Suppress("DEPRECATION")
            super.onActivityResult(requestCode, resultCode, data)
            return
        }
        val uris = if (resultCode == RESULT_OK && data != null) collectUris(data) else emptyList()
        if (uris.isEmpty()) {
            // Cancelled: leave the cache alone and report an empty list.
            native { nativeImagesPicked(emptyArray(), 0) }
            return
        }
        // Lets Rust show the loading overlay if copying takes long; the picker itself shows none.
        native { nativePickCopying() }
        runOnIo({ native { nativePickFailed() } }) {
            val copied = copyToCache(uris)
            native { nativeImagesPicked(copied.paths.toTypedArray(), copied.failed) }
        }
    }

    /** Multi-select results come in `clipData`, a single pick may arrive in `data` only. */
    private fun collectUris(data: Intent): List<Uri> {
        val clip = data.clipData
        val fromClip = if (clip == null) emptyList() else (0 until clip.itemCount).mapNotNull { clip.getItemAt(it).uri }
        return fromClip.ifEmpty { listOfNotNull(data.data) }
    }

    /**
     * Copies each URI to `cacheDir/picked/img-<index>` (app-private, never named after the URI) and
     * returns the paths of the copies that succeeded. Runs on [io].
     */
    private data class Copied(val paths: List<String>, val failed: Int)

    private fun copyToCache(uris: List<Uri>): Copied {
        val dir = File(cacheDir, "picked")
        dir.deleteRecursively()
        if (!dir.mkdirs()) {
            Log.e(TAG, "cannot create cache dir")
            return Copied(emptyList(), uris.size)
        }
        val paths = ArrayList<String>()
        var failed = 0
        uris.forEachIndexed { index, uri ->
            val target = File(dir, "img-$index")
            val temp = File(dir, "img-$index.tmp")
            try {
                val mime = contentResolver.getType(uri)
                if (mime == "image/jpeg" || mime == "image/png") {
                    val input = contentResolver.openInputStream(uri) ?: throw IOException("no stream")
                    input.use { source -> target.outputStream().use { source.copyTo(it) } }
                    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    BitmapFactory.decodeFile(target.absolutePath, bounds)
                    if (bounds.outWidth > 0 && bounds.outHeight > 0 && fitScale(bounds.outWidth, bounds.outHeight) < 1.0) {
                        transcode(ImageDecoder.createSource(target), temp, index)
                        if (!target.delete() || !temp.renameTo(target)) throw IOException("cannot replace cache image")
                    }
                } else {
                    transcode(ImageDecoder.createSource(contentResolver, uri), target, index)
                    Log.i(TAG, "transcoded image $index")
                }
                paths.add(target.absolutePath)
            } catch (e: Exception) {
                Log.e(TAG, "image $index failed: ${e.javaClass.simpleName}")
                failed++
                target.delete()
                temp.delete()
            } catch (e: OutOfMemoryError) {
                Log.e(TAG, "image $index needs too much memory")
                failed++
                target.delete()
                temp.delete()
            }
        }
        return Copied(paths, failed)
    }

    /** Decode on [io] and write an upright sRGB image that Rust can read. */
    private fun transcode(source: ImageDecoder.Source, target: File, index: Int) {
        var retryScale = 1.0
        var bitmap: Bitmap? = null
        for (attempt in 0..MAX_DECODE_RETRIES) {
            try {
                bitmap = decodeForTranscode(source, retryScale, index)
                break
            } catch (e: IOException) {
                // ImageDecoder can wrap native allocation failures in IOException rather than OOM.
                if (attempt == MAX_DECODE_RETRIES || e.message?.contains("out of memory", ignoreCase = true) != true) throw e
            } catch (e: OutOfMemoryError) {
                if (attempt == MAX_DECODE_RETRIES) throw e
            }
            retryScale *= 0.5
            Log.w(TAG, "image $index decode ran out of memory; retrying at scale $retryScale")
        }
        val decoded = bitmap ?: throw IOException("image decode failed")
        try {
            val format = if (decoded.hasAlpha()) Bitmap.CompressFormat.PNG else Bitmap.CompressFormat.JPEG
            target.outputStream().use {
                if (!decoded.compress(format, JPEG_QUALITY, it)) throw IOException("image compression failed")
            }
        } finally {
            decoded.recycle()
        }
    }

    private fun decodeForTranscode(source: ImageDecoder.Source, retryScale: Double, index: Int): Bitmap {
        var downscaled = false
        val bitmap = ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
            decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            decoder.setTargetColorSpace(ColorSpace.get(ColorSpace.Named.SRGB))
            val scale = fitScale(info.size.width, info.size.height) * retryScale
            if (scale < 1.0) {
                val width = floor(info.size.width * scale).toInt().coerceAtLeast(1)
                val height = floor(info.size.height * scale).toInt().coerceAtLeast(1)
                decoder.setTargetSize(width, height)
                downscaled = true
            }
        }
        if (downscaled) Log.i(TAG, "downscaled image $index to ${bitmap.width}x${bitmap.height}")
        return bitmap
    }

    private fun fitScale(width: Int, height: Int): Double {
        val pixels = width.toLong() * height.toLong()
        return min(1.0, min(sqrt(MAX_PIXELS.toDouble() / pixels), min(MAX_EDGE.toDouble() / width, MAX_EDGE.toDouble() / height)))
    }

    private companion object {
        const val TAG = "dimuli"
        const val REQUEST_PICK = 1
        const val KEY_COLLAGE_OPEN = "dimuli.collage_open"
        /** Mirrors DECODE_MAX_PIXELS in collage-core/src/decode.rs. */
        const val MAX_PIXELS = 64_000_000L
        /** Mirrors DECODE_MAX_EDGE in collage-core/src/decode.rs. */
        const val MAX_EDGE = 16_384
        const val JPEG_QUALITY = 95
        const val MAX_DECODE_RETRIES = 3
    }
}
