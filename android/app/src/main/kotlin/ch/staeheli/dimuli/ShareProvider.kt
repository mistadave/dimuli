package ch.staeheli.dimuli

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import java.io.File
import java.io.FileNotFoundException

/**
 * Serves exactly one file, `<cacheDir>/share/dimuli-collage.jpg`, read-only (D-09: no AndroidX
 * FileProvider). Not exported; receivers get access only through the temporary read grant of the
 * share intent. The URI is compared as a whole, no path is ever derived from it.
 */
class ShareProvider : ContentProvider() {
    override fun onCreate() = true

    private fun isServed(uri: Uri) = uri == uri()

    private fun servedFile(): File {
        val dir = context?.cacheDir ?: throw FileNotFoundException("no context")
        return File(File(dir, "share"), FILE_NAME)
    }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (!isServed(uri)) throw FileNotFoundException("unknown uri")
        if (mode != "r") throw SecurityException("read-only provider")
        return ParcelFileDescriptor.open(servedFile(), ParcelFileDescriptor.MODE_READ_ONLY)
    }

    override fun query(
        uri: Uri,
        projection: Array<String>?,
        selection: String?,
        selectionArgs: Array<String>?,
        sortOrder: String?,
    ): Cursor {
        if (!isServed(uri)) throw FileNotFoundException("unknown uri")
        val columns = projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
        val row = columns.map { column ->
            when (column) {
                OpenableColumns.DISPLAY_NAME -> FILE_NAME
                OpenableColumns.SIZE -> servedFile().length()
                else -> null
            }
        }
        return MatrixCursor(columns, 1).apply { addRow(row) }
    }

    override fun getType(uri: Uri): String? = if (isServed(uri)) "image/jpeg" else null

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<String>?) = 0

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<String>?) = 0

    companion object {
        const val FILE_NAME = "dimuli-collage.jpg"

        fun uri(): Uri = Uri.parse("content://ch.staeheli.dimuli.share/$FILE_NAME")
    }
}
