package io.github.cavestory_rs;

import java.io.*;
import java.nio.file.*;
import java.util.zip.*;

/** Host JVM checks: no device, user data, or Android mocks required. */
public final class ChineseDataInstallerTest {
    private static void check(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }
    private static byte[] zip(String name) throws Exception {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        try (ZipOutputStream zip = new ZipOutputStream(out)) {
            zip.putNextEntry(new ZipEntry(name)); zip.write(1); zip.closeEntry();
        }
        return out.toByteArray();
    }
    private static void rejects(Checked action) throws Exception {
        try { action.run(); } catch (IOException expected) { return; }
        throw new AssertionError("unsafe or incomplete input accepted");
    }
    interface Checked { void run() throws Exception; }
    public static void main(String[] args) throws Exception {
        Path root = Files.createTempDirectory("cavestory-installer-test-");
        try {
            for (String name : new String[]{"../escape", "/escape", "Doukutsu/data/../../escape", "Doukutsu/data/a\\b", "C:/escape"}) {
                rejects(() -> ChineseDataInstaller.unpack(new ByteArrayInputStream(zip(name)), root.resolve("unsafe").toFile()));
            }
            rejects(() -> ChineseDataInstaller.verify(new byte[]{1,2,3}));
            rejects(() -> ChineseDataInstaller.validate(root.toFile()));
            File existing = root.resolve("data").toFile(); existing.mkdir();
            Files.writeString(existing.toPath().resolve("user.txt"), "keep");
            rejects(() -> ChineseDataInstaller.publish(root.resolve("staged").toFile(), existing));
            check(Files.readString(existing.toPath().resolve("user.txt")).equals("keep"), "existing data changed");
            Thread.currentThread().interrupt();
            rejects(() -> ChineseDataInstaller.checkCancelled());
            Thread.interrupted();
            File staged = root.resolve("ready").toFile(); staged.mkdir();
            Files.writeString(staged.toPath().resolve("marker"), "complete");
            File destination = root.resolve("installed").toFile();
            ChineseDataInstaller.publish(staged, destination);
            check(new File(destination, "marker").isFile() && !staged.exists(), "atomic publish");
            File abandoned = root.resolve(".chinese-install-00000000-0000-0000-0000-000000000000").toFile();
            abandoned.mkdir();
            Files.writeString(abandoned.toPath().resolve("partial"), "unfinished");
            ChineseDataInstaller.removeAbandoned(root.toFile());
            check(!abandoned.exists() && existing.exists(), "restart cleanup must preserve user data");
            // Android commonly reaches /data/data through the /data/user/0 alias.
            Path alias = root.resolve("alias");
            Files.createSymbolicLink(alias, root);
            File partial = root.resolve(".chinese-install-11111111-1111-1111-1111-111111111111").toFile();
            new File(partial, "nested").mkdirs();
            Files.writeString(partial.toPath().resolve("nested/partial"), "unfinished");
            ChineseDataInstaller.removeAbandoned(alias.toFile());
            check(!partial.exists(), "cleanup through aliased Android files path");
            Files.delete(alias);
            if (args.length == 2) {
                byte[] archive = Files.readAllBytes(Path.of(args[0]));
                ChineseDataInstaller.verify(archive);
                File output = root.resolve("real").toFile();
                ChineseDataInstaller.unpack(new ByteArrayInputStream(archive), output);
                ChineseDataInstaller.copyOverlay(new FileInputStream(args[1]), output);
                ChineseDataInstaller.validate(output);
                check(new File(output, "Readme.txt").isFile(), "translation attribution missing");
                check(new File(output, "Stage/Start.tsc").isFile(), "story missing");
            }
            System.out.println("Chinese installer: safety, cancellation, preservation, atomic publish and supplied archive passed");
        } finally {
            try (var paths = Files.walk(root)) {
                for (Path path : paths.sorted(java.util.Comparator.reverseOrder()).toList()) Files.delete(path);
            }
        }
    }
}
