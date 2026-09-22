package io.github.cavestory_rs;

import java.io.*;
import java.nio.file.*;
import java.util.Arrays;

/** Real archive and asset source checks; all files live under the supplied runs directory. */
public final class JapaneseDataInstallerTest {
    interface Checked { void run() throws Exception; }
    static void rejects(Checked action) throws Exception {
        try { action.run(); } catch (IOException expected) { return; }
        throw new AssertionError("unsafe operation accepted");
    }
    public static void main(String[] args) throws Exception {
        Path root = Files.createTempDirectory(Path.of(args[3]), "japanese-installer-");
        try {
            byte[] archive = Files.readAllBytes(Path.of(args[0]));
            byte[] bundled = ChineseDataInstaller.loadArchive("dou_1006.zip", name -> new ByteArrayInputStream(archive), (n,t) -> {});
            if (!Arrays.equals(archive, bundled)) throw new AssertionError("offline asset changed");
            rejects(() -> ChineseDataInstaller.loadArchive("dou_1006.zip", name -> new ByteArrayInputStream(new byte[]{1}), (n,t) -> {}));
            for (String name : new String[]{"dou_sc102.zip", "cavestoryen.zip"}) {
                byte[] original = Files.readAllBytes(Path.of(args[0]).resolveSibling(name));
                byte[] loaded = ChineseDataInstaller.loadArchive(name, asset -> new ByteArrayInputStream(original), (n,t) -> {});
                if (!Arrays.equals(original, loaded)) throw new AssertionError("offline resource changed: " + name);
            }
            rejects(() -> ChineseDataInstaller.loadArchive("unknown.zip", name -> new ByteArrayInputStream(archive), (n,t) -> {}));
            Thread.currentThread().interrupt();
            rejects(() -> ChineseDataInstaller.loadArchive("dou_1006.zip", name -> new ByteArrayInputStream(archive), (n,t) -> {}));
            Thread.interrupted();
            Path data = root.resolve("data"); Files.createDirectories(data);
            if (ChineseDataInstaller.needsJapanese(data.toFile())) throw new AssertionError("unmanaged install adopted");
            Files.writeString(data.resolve("chinese-install.json"), "{}");
            Files.writeString(data.resolve("Profile.dat"), "saved game");
            Files.writeString(data.resolve("settings.json"), "explicit language");
            Files.createDirectory(data.resolve("en"));
            if (!ChineseDataInstaller.needsJapanese(data.toFile())) throw new AssertionError("bilingual upgrade missed");
            Files.createDirectory(data.resolve("jp"));
            if (ChineseDataInstaller.needsJapanese(data.toFile())) throw new AssertionError("existing empty custom directory adopted");
            Files.delete(data.resolve("jp"));
            File staged = root.resolve("staged").toFile();
            ChineseDataInstaller.prepareJapanese(archive, new FileInputStream(args[1]), new FileInputStream(args[2]), staged);
            for (String name : new String[]{"Doukutsu.exe", "Head.tsc", "ArmsItem.tsc", "Credit.tsc", "Stage/Start.tsc", "Readme.txt", "locale/jp.json", "fonts/japanese/chinese-12.fnt", "japanese-install.json"})
                if (!new File(staged, name).isFile()) throw new AssertionError("missing " + name);
            if (new File(staged, "Config.dat").exists()) throw new AssertionError("old config imported");
            Thread.currentThread().interrupt();
            rejects(() -> ChineseDataInstaller.publish(staged, data.resolve("jp").toFile()));
            Thread.interrupted();
            if (data.resolve("jp").toFile().exists()) throw new AssertionError("cancel published data");
            ChineseDataInstaller.publish(staged, data.resolve("jp").toFile());
            if (ChineseDataInstaller.needsJapanese(data.toFile())) throw new AssertionError("repeat upgrade");
            rejects(() -> ChineseDataInstaller.publish(root.resolve("unused").toFile(), data.resolve("jp").toFile()));
            if (!Files.readString(data.resolve("Profile.dat")).equals("saved game") || !Files.readString(data.resolve("settings.json")).equals("explicit language")) throw new AssertionError("user state changed");
            archive[0] ^= 1;
            rejects(() -> ChineseDataInstaller.prepareJapanese(archive, new FileInputStream(args[1]), new FileInputStream(args[2]), root.resolve("bad").toFile()));
            if (root.resolve("bad").toFile().exists()) throw new AssertionError("corrupt archive staged");
            if (args.length > 4 && args[4].equals("network")) {
                byte[] downloaded = ChineseDataInstaller.loadArchive("dou_1006.zip", name -> { throw new FileNotFoundException(name); }, (n,t) -> {});
                ChineseDataInstaller.verify(downloaded, ChineseDataInstaller.JAPANESE_SHA256);
                System.out.println("Japanese installer: absent bundled asset downloads pinned official archive successfully");
            }
            System.out.println("Japanese installer: offline assets, real archive, attribution, fonts, save/settings protection, cancellation, existing directory and corrupt archive passed");
        } finally { ChineseDataInstaller.removeStaging(root.toFile()); }
    }
}
