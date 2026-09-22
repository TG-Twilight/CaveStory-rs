package io.github.cavestory_rs;

import java.nio.file.*;
import java.util.Locale;

public final class StartupPolicyTest {
    static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
    public static void main(String[] args) throws Exception {
        for (String tag : new String[]{"zh-CN", "zh-SG", "zh-MY", "zh-Hans", "zh-Hans-HK", "zh"})
            check(GameLanguage.code(Locale.forLanguageTag(tag)).equals("zh-Hans"), tag);
        for (String tag : new String[]{"ja-JP", "ja"})
            check(GameLanguage.code(Locale.forLanguageTag(tag)).equals("jp"), tag);
        for (String tag : new String[]{"en-US", "de-DE", "fr-FR", "zh-TW", "zh-HK", "zh-MO", "zh-Hant-CN", "ko-KR", "ar"})
            check(GameLanguage.code(Locale.forLanguageTag(tag)).equals("en"), tag);
        Path root = Files.createTempDirectory(Path.of(args[0]), "startup-");
        try {
            check(StartupResources.inspect(root.toFile()) == StartupResources.State.EMPTY, "empty installation");
            Files.createDirectories(root.resolve("data/locale"));
            Files.writeString(root.resolve("data/locale/en.json"), "{}");
            check(StartupResources.inspect(root.toFile()) == StartupResources.State.INVALID, "support files are not game data");
            for (String name : new String[]{"Head.tsc", "ArmsItem.tsc", "Stage/Start.pxm", "Stage/Start.tsc"}) {
                Path file = root.resolve("data/" + name); Files.createDirectories(file.getParent()); Files.writeString(file, "fixture");
            }
            Files.writeString(root.resolve("data/Doukutsu.exe"), "extractable original");
            check(StartupResources.inspect(root.toFile()) == StartupResources.State.READY, "nested freeware with original exe");
            Files.delete(root.resolve("data/Doukutsu.exe"));
            Files.writeString(root.resolve("data/stage.sect"), "extracted table");
            check(StartupResources.inspect(root.toFile()) == StartupResources.State.READY, "extracted data");
            Files.delete(root.resolve("data/Stage/Start.tsc"));
            check(StartupResources.inspect(root.toFile()) == StartupResources.State.INVALID, "incomplete resource tree");
        } finally { ChineseDataInstaller.removeStaging(root.toFile()); }
        System.out.println("Startup resource detection and Android locale mapping passed");
    }
}
