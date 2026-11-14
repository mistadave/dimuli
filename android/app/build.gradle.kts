plugins { id("com.android.application") }

android {
    namespace = "ch.staeheli.dimuli"
    compileSdk = 37
    buildToolsVersion = "37.0.0"
    ndkVersion = "30.0.16248370"

    defaultConfig {
        applicationId = "ch.staeheli.dimuli"
        minSdk = 33
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// cargo ndk -> src/main/jniLibs before every build; -PrustAbis=x86_64 for emulator-only builds.
val rustAbis = (findProperty("rustAbis") as String? ?: "arm64-v8a,x86_64").split(",")
val sdkDir = androidComponents.sdkComponents.sdkDirectory
val jniLibsDir = layout.projectDirectory.dir("src/main/jniLibs").asFile

val buildRust = tasks.register<Exec>("buildRust") {
    group = "build"
    description = "Builds libdimuli.so with cargo ndk"
    workingDir = rootDir.parentFile // repository root
    doFirst {
        // An APK must never carry a stale .so of an ABI that was not rebuilt.
        jniLibsDir.deleteRecursively()
        // android-activity's Java glue needs the platform jar (cargo-ndk does not set it).
        environment("ANDROID_JAR", sdkDir.get().file("platforms/android-37.0/android.jar").asFile.path)
    }
    commandLine(
        listOf("cargo", "ndk") + rustAbis.flatMap { listOf("-t", it) } +
            listOf("--platform", "33", "-o", jniLibsDir.path, "build", "-p", "dimuli-app", "--lib", "--release")
    )
}

tasks.named("preBuild") { dependsOn(buildRust) }
