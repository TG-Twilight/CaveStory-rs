plugins {
    id("com.android.application") version "8.9.1" apply false
}

allprojects {
    val runs = File(System.getenv("CAVESTORY_RUNS") ?: File(rootProject.projectDir.parentFile.parentFile, "CaveStory-rs-runs").absolutePath)
    layout.buildDirectory.set(File(runs, "cache/android/${project.name}/build"))
    repositories {
        google()
        mavenCentral()
    }
}
