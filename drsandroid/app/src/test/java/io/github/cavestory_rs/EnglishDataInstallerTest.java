package io.github.cavestory_rs;

import java.io.*;
import java.nio.file.*;
import java.util.Arrays;

/** Host test using the actual pinned external archive; never touches device data. */
public final class EnglishDataInstallerTest {
    public static void main(String[] args) throws Exception {
        Path root = Files.createTempDirectory("english-installer-test");
        try {
            File data = root.resolve("data").toFile();
            data.mkdir();
            if (ChineseDataInstaller.needsEnglish(data)) throw new AssertionError("unmanaged data");
            Files.writeString(data.toPath().resolve("chinese-install.json"), "{}");
            Files.writeString(data.toPath().resolve("Profile.dat"), "player progress");
            if (!ChineseDataInstaller.needsEnglish(data)) throw new AssertionError("missing upgrade");
            byte[] archive = Files.readAllBytes(Path.of(args[0]));
            byte[] save = Files.readAllBytes(data.toPath().resolve("Profile.dat"));
            File staging = root.resolve("staging").toFile();
            ChineseDataInstaller.prepareEnglish(archive, new FileInputStream(args[1]), staging);
            if (new File(staging, "Config.dat").exists()) throw new AssertionError("legacy settings imported");
            ChineseDataInstaller.publish(staging, new File(data, "en"));
            if (ChineseDataInstaller.needsEnglish(data)) throw new AssertionError("repeated upgrade");
            if (!Arrays.equals(save, Files.readAllBytes(data.toPath().resolve("Profile.dat")))) throw new AssertionError("save changed");
            try {
                ChineseDataInstaller.publish(root.resolve("unused").toFile(), new File(data, "en"));
                throw new AssertionError("existing resources overwritten");
            } catch (IOException expected) {}
            archive[0] ^= 1;
            try {
                ChineseDataInstaller.prepareEnglish(archive, new ByteArrayInputStream(new byte[0]), root.resolve("bad").toFile());
                throw new AssertionError("corrupt archive accepted");
            } catch (IOException expected) {}
            if (root.resolve("bad").toFile().exists()) throw new AssertionError("partial corrupt install");
            if (args.length == 4) {
                java.util.Properties fingerprints = new java.util.Properties();
                try (InputStream input = new FileInputStream(args[2])) { fingerprints.load(input); }
                Path legacy = root.resolve("legacy");
                Files.createDirectories(legacy.resolve("locale"));
                Files.createDirectories(legacy.resolve("fonts"));
                Files.writeString(legacy.resolve("locale/zh-Hans.json"), "{}");
                Files.writeString(legacy.resolve("fonts/chinese-12.fnt"), "font fixture");
                for (String name : fingerprints.stringPropertyNames()) {
                    Files.createDirectories(legacy.resolve(name).getParent());
                    Files.copy(Path.of(args[3]).resolve(name), legacy.resolve(name));
                }
                if (!ChineseDataInstaller.needsEnglish(legacy.toFile(), new FileInputStream(args[2]))) throw new AssertionError("legacy copy missed");
                Files.createDirectory(legacy.resolve("en"));
                if (!ChineseDataInstaller.needsJapanese(legacy.toFile(), new FileInputStream(args[2]))) throw new AssertionError("legacy bilingual Japanese upgrade missed");
                Files.delete(legacy.resolve("en"));
                Files.writeString(legacy.resolve("Head.tsc"), "custom story");
                if (ChineseDataInstaller.needsEnglish(legacy.toFile(), new FileInputStream(args[2]))) throw new AssertionError("modified game recognized as original");
                if (ChineseDataInstaller.needsJapanese(legacy.toFile(), new FileInputStream(args[2]))) throw new AssertionError("modified legacy Japanese game adopted");
                System.out.println("Legacy import: exact 400 fingerprints accepted, modified story protected");
            }
            System.out.println("English installer: real archive, attribution, saves, existing data and corrupt download passed");
        } finally {
            try (var paths = Files.walk(root)) {
                for (Path path : paths.sorted(java.util.Comparator.reverseOrder()).toList()) Files.delete(path);
            }
        }
    }
}
