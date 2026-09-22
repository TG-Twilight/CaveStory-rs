package io.github.cavestory_rs;

import java.io.*;
import java.nio.file.*;
import java.util.Arrays;
import java.util.zip.ZipFile;

/** Cross-check the real Java installer against the separately generated Python output. */
public final class DistributionScriptsTest {
    public static void main(String[] args) throws Exception {
        Path runs = Paths.get(args[0]), assets = Paths.get(args[1]), expected = Paths.get(args[2]);
        String[][] sources = {{"zh-Hans", "dou_sc102.zip", "Doukutsu"}, {"en", "cavestoryen.zip", "CaveStory"}, {"jp", "dou_1006.zip", "doukutsu"}};
        for (String[] source : sources) {
            byte[] original;
            try (ZipFile archive = new ZipFile(runs.resolve("downloads").resolve(source[1]).toFile())) {
                original = DistributionScripts.read(archive.getInputStream(archive.getEntry(source[2] + "/data/Stage/Barr.tsc")));
            }
            ChineseDataInstaller.Assets loader = name -> Files.newInputStream(assets.resolve(name));
            byte[] patched = DistributionScripts.patch(original, source[0], loader);
            check(Arrays.equals(patched, Files.readAllBytes(expected.resolve(source[0] + ".bin"))), "Java and Python script results differ: " + source[0]);
            check(Arrays.equals(DistributionScripts.patch(patched, source[0], loader), patched), "patch is not idempotent");
            byte[] custom = original.clone(); custom[0] ^= 1;
            check(Arrays.equals(DistributionScripts.patch(custom, source[0], loader), custom), "custom script changed");
            Path directory = Files.createTempDirectory(runs.resolve("maintenance"), "distribution-host-");
            try {
                Path script = directory.resolve("Stage/Barr.tsc");
                Files.createDirectories(script.getParent()); Files.write(script, original);
                DistributionScripts.install(directory.toFile(), source[0], loader);
                check(Arrays.equals(Files.readAllBytes(script), patched), "published script differs");
                check(Arrays.equals(Files.readAllBytes(directory.resolve(".distribution-notice/Barr.tsc.original")), original), "original backup differs");
                DistributionScripts.install(directory.toFile(), source[0], loader);
                byte[] previous = Files.readAllBytes(expected.resolve(source[0] + ".previous.bin"));
                Files.write(script, previous);
                DistributionScripts.install(directory.toFile(), source[0], loader);
                check(Arrays.equals(Files.readAllBytes(script), patched), "previous release was not upgraded: " + source[0]);
                check(Arrays.equals(Files.readAllBytes(directory.resolve(".distribution-notice/Barr.tsc.original")), original), "upgrade replaced original backup");
                byte[] editedPrevious = previous.clone(); editedPrevious[editedPrevious.length - 1] ^= 1;
                Files.write(script, editedPrevious);
                DistributionScripts.install(directory.toFile(), source[0], loader);
                check(Arrays.equals(Files.readAllBytes(script), editedPrevious), "user edit of previous notice changed");
                byte[] largeCustom = new byte[130 * 1024];
                Arrays.fill(largeCustom, (byte) 'A');
                Files.write(script, largeCustom);
                DistributionScripts.install(directory.toFile(), source[0], loader);
                check(Arrays.equals(Files.readAllBytes(script), largeCustom), "large custom script blocked or changed");
            } finally {
                try (var paths = Files.walk(directory)) {
                    for (Path path : paths.sorted(java.util.Comparator.reverseOrder()).toArray(Path[]::new)) Files.delete(path);
                }
            }
        }
        System.out.println("Distribution scripts: three languages, cross-implementation, custom preservation, backup and repeated install passed");
    }
    private static void check(boolean value, String message) { if (!value) throw new AssertionError(message); }
}
