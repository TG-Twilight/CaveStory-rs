import java.security.KeyStore
import java.time.LocalDate
import java.time.format.DateTimeFormatter

plugins {
    id("com.android.application")
}

// Keep the pinned SDL checkout pristine; only HIDDeviceManager has a local override.
val prepareSdlJava by tasks.registering(Sync::class) {
    from("src/main/cpp/SDL2/android-project/app/src/main/java") {
        exclude("org/libsdl/app/HIDDeviceManager.java")
    }
    from("src/main/sdl") { include("**/*.java") }
    into(layout.buildDirectory.dir("generated/sdlJava"))
    doLast {
        // Guard task restoration before SDL loads native code, including Android 7.
        // Patch the generated copy only; keep the pinned SDL checkout pristine.
        val activity = destinationDir.resolve("org/libsdl/app/SDLActivity.java")
        var source = activity.readText().replace("\r\n", "\n")
        val declaration = "    @Override\n    protected void onCreate(Bundle savedInstanceState) {"
        val startup = "        super.onCreate(savedInstanceState);"
        require(source.split(declaration).size == 2 && source.split(startup).size == 2) { "SDL startup hook no longer matches" }
        source = source.replace(declaration, "    protected boolean onBeforeNativeStart() { return true; }\n\n" + declaration)
        source = source.replace(startup, startup + "\n        if (!onBeforeNativeStart()) { mBrokenLibraries = true; mSingleton = this; return; }")
        activity.writeText(source)
    }
}

tasks.named("preBuild") { dependsOn(prepareSdlJava) }

// Shared support assets contain fonts and locales. The game package additionally bundles verified archives.
val chineseAssets = layout.buildDirectory.dir("generated/chineseAssets")
val prepareChineseAssets by tasks.registering(Exec::class) {
    val repository = rootProject.projectDir.parentFile.parentFile
    inputs.files(fileTree(File(repository, "res/chinese/locale")))
    inputs.file(File(repository, "res/chinese/legacy-resource-hashes.properties"))
    inputs.files(File(repository, "tools/install_japanese_resources.py"), File(repository, "tools/install_english_resources.py"))
    inputs.files(File(repository, "tools/build_chinese_font.py"), File(repository, "tools/prepare_android_chinese_assets.py"))
    inputs.file(File(repository, "tools/patch_distribution_notice.py"))
    inputs.files(fileTree(File(repository, "res/distribution")))
    inputs.files(File(repository, "src/data/builtin/builtin_data/locale/en.json"), File(repository, "src/data/builtin/builtin_data/locale/jp.json"))
    outputs.dir(chineseAssets)
    workingDir(repository)
    commandLine(if (System.getProperty("os.name").startsWith("Windows")) "py" else "python3",
        "tools/prepare_android_chinese_assets.py", "--output", chineseAssets.get().asFile.absolutePath)
}
tasks.named("preBuild") { dependsOn(prepareChineseAssets) }

// Both local variants use the user's existing identity; never fall back to a debug key.
val signingPath = System.getenv("REVIA_KS_PATH")
    ?: "D:/Project/CodeX/Mod/OPCameraPro/秋风のとおり道.jks"
val signingPassword = System.getenv("REVIA_KS_PASS")
    ?: error("Set REVIA_KS_PASS (Windows: use tools/build_android.ps1)")
val signingStore = KeyStore.getInstance("JKS").apply {
    file(signingPath).inputStream().use { load(it, signingPassword.toCharArray()) }
}
val signingAlias = signingStore.aliases().toList().single { signingStore.isKeyEntry(it) }

val buildDate = providers.gradleProperty("caveStoryBuildDate")
    .orElse(providers.environmentVariable("CAVESTORY_BUILD_VERSION"))
    .orElse(LocalDate.now().format(DateTimeFormatter.BASIC_ISO_DATE)).get()
require(buildDate.matches(Regex("[0-9]{8}"))) { "Build date must be YYYYMMDD" }
LocalDate.parse(buildDate, DateTimeFormatter.BASIC_ISO_DATE)
val bundledGameAssets = providers.gradleProperty("caveStoryGameAssets")

android {
    namespace = "io.github.cavestory_rs"

    /**
     * NOTE: If you want to change the versions of packages required for the build
     * (e.g. build tools, compile SDK, NDK) or add new ones,
     * make the appropriate changes in packages.txt. Otherwise CI could break.
     */
    compileSdk = 35
    buildToolsVersion = "35.0.1"
    ndkVersion = "28.0.13004108"

    defaultConfig {
        applicationId = "io.github.cavestory_rs"
        minSdk = 24
        targetSdk = 35
        versionCode = buildDate.toInt()
        versionName = buildDate

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            stl = "c++_shared"
        }

        externalNativeBuild {
            cmake {
                arguments.add("-DANDROID_STL=c++_shared")
            }
        }

        val documentsAuthorityValue = "$applicationId.documents"
        manifestPlaceholders["documentsAuthority"] = documentsAuthorityValue
        buildConfigField("String", "DOCUMENTS_AUTHORITY", "\"$documentsAuthorityValue\"")

        resValue("string", "app_name", "CaveStory-rs")
    }

    signingConfigs {
        create("revia") {
            storeFile = file(signingPath)
            storePassword = signingPassword
            keyAlias = signingAlias
            keyPassword = signingPassword
            enableV1Signing = true
            enableV2Signing = true
        }
    }
    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("revia")
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )

            ndk {
                stl = "c++_shared"
            }

            packaging {
                resources {
                    excludes.add("**/DebugProbesKt.bin")
                }
            }
        }
        debug {
            signingConfig = signingConfigs.getByName("revia")
            // Local clean-install validation can use a separate application sandbox.
            applicationIdSuffix = providers.gradleProperty("caveStoryTestSuffix").orElse(".debug").get()
            resValue("string", "app_name", "CaveStory-rs (debug)")

            isJniDebuggable = true

            val applicationId = defaultConfig.applicationId!!
            val documentsAuthorityValue = "$applicationId$applicationIdSuffix.documents"
            manifestPlaceholders["documentsAuthority"] = documentsAuthorityValue
            buildConfigField("String", "DOCUMENTS_AUTHORITY", "\"$documentsAuthorityValue\"")
        }
    }

    // Deliver two independent ARM APKs in a single build, for both variants.
    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "armeabi-v7a")
            isUniversalApk = false
        }
    }

    buildFeatures {
        buildConfig = true
    }

    sourceSets {
        getByName("main") {
            java.srcDir(prepareSdlJava.map { it.destinationDir })
            assets.srcDir(chineseAssets)
            assets.srcDir(File(rootProject.projectDir.parentFile.parentFile, "vendor/trainer/notices"))
            if (bundledGameAssets.isPresent) assets.srcDir(file(bundledGameAssets.get()))
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        viewBinding = true
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            buildStagingDirectory = File(layout.buildDirectory.get().asFile.parentFile, "cxx")
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

dependencies {
    implementation("dev.rikka.shizuku:api:13.1.5")
    implementation("dev.rikka.shizuku:provider:13.1.5")
    implementation("androidx.annotation:annotation:1.5.0")
    implementation("androidx.appcompat:appcompat:1.6.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.1")
    implementation("androidx.core:core:1.9.0")
    implementation("com.google.android.material:material:1.8.0")
}
