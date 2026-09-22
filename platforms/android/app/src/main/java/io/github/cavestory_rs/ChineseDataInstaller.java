package io.github.cavestory_rs;

import java.io.*;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HashSet;
import java.util.Set;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

/** Installs only into a private staging directory, then publishes one complete data tree. */
final class ChineseDataInstaller {
    static final String SOURCE = "https://www.cavestory.one/downloads/dou_sc102.zip";
    static final String SHA256 = "d1e632a8f88cbd704ad300a27fd74abe2ada8faa67789d2274dd8ef8fee4df1e";
    static final String ENGLISH_SOURCE = "https://www.cavestory.one/downloads/cavestoryen.zip";
    static final String ENGLISH_SHA256 = "aa87fa30bee9b4980640c7e104791354e0f1f6411ee0d45a70af70046aa0685f";
    static final String JAPANESE_SOURCE = "https://studiopixel.jp/binaries/dou_1006.zip";
    static final String JAPANESE_SHA256 = "68d0dfab0afa2bfb0c6f750638d6a8883576cbbe33f65be8ff51d74272d0eea0";
    private static final int MAX_DOWNLOAD = 2 * 1024 * 1024;
    private static final int MAX_UNPACKED = 16 * 1024 * 1024;
    interface Progress { void update(int downloaded, int total); }
    interface Assets { InputStream open(String name) throws IOException; }

    /** A present but damaged bundled archive must fail closed, never silently use the network. */
    static byte[] loadArchive(String name, Assets assets, Progress progress) throws IOException {
        String source, hash;
        switch (name) {
            case "dou_sc102.zip": source = SOURCE; hash = SHA256; break;
            case "cavestoryen.zip": source = ENGLISH_SOURCE; hash = ENGLISH_SHA256; break;
            case "dou_1006.zip": source = JAPANESE_SOURCE; hash = JAPANESE_SHA256; break;
            default: throw new IOException("Unknown resource archive");
        }
        checkCancelled();
        InputStream asset;
        try { asset = assets.open("game/" + name); }
        catch (FileNotFoundException absent) { return download(source, hash, progress); }
        try (InputStream input = asset; ByteArrayOutputStream out = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[16384];
            int count;
            while ((count = input.read(buffer)) != -1) {
                checkCancelled();
                if (out.size() + count > MAX_DOWNLOAD) throw new IOException("Bundled archive size limit exceeded");
                out.write(buffer, 0, count);
            }
            byte[] bytes = out.toByteArray();
            verify(bytes, hash);
            return bytes;
        }
    }

    static void checkCancelled() throws InterruptedIOException {
        if (Thread.currentThread().isInterrupted()) throw new InterruptedIOException("Cancelled");
    }

    static byte[] download(Progress progress) throws IOException {
        return download(SOURCE, SHA256, progress);
    }

    static byte[] downloadEnglish(Progress progress) throws IOException {
        return download(ENGLISH_SOURCE, ENGLISH_SHA256, progress);
    }

    private static byte[] download(String source, String expectedHash, Progress progress) throws IOException {
        URL url = new URL(source);
        for (int redirect = 0; redirect <= 3; redirect++) {
            checkCancelled();
            if (!"https".equals(url.getProtocol()) || !("www.cavestory.one".equals(url.getHost())
                    || "cavestory.one".equals(url.getHost()) || "studiopixel.jp".equals(url.getHost())) || url.getUserInfo() != null
                    || (url.getPort() != -1 && url.getPort() != 443)) {
                throw new IOException("Unexpected download redirect");
            }
            HttpURLConnection connection = (HttpURLConnection) url.openConnection();
            connection.setConnectTimeout(15000);
            connection.setReadTimeout(15000);
            connection.setInstanceFollowRedirects(false);
            connection.setRequestProperty("Accept-Encoding", "identity");
            try {
                int status = connection.getResponseCode();
                if (status == 301 || status == 302 || status == 303 || status == 307 || status == 308) {
                    String location = connection.getHeaderField("Location");
                    if (location == null) throw new IOException("Missing redirect location");
                    url = new URL(url, location);
                    continue;
                }
                if (status != 200) throw new IOException("HTTP " + status);
                int total = connection.getContentLength();
                if (total > MAX_DOWNLOAD || total == 0) throw new IOException("Unexpected archive size");
                try (InputStream input = connection.getInputStream(); ByteArrayOutputStream out = new ByteArrayOutputStream()) {
                    byte[] buffer = new byte[16384];
                    int count;
                    while ((count = input.read(buffer)) != -1) {
                        checkCancelled();
                        if (out.size() + count > MAX_DOWNLOAD) throw new IOException("Download size limit exceeded");
                        out.write(buffer, 0, count);
                        progress.update(out.size(), total);
                    }
                    if (total >= 0 && out.size() != total) throw new IOException("Incomplete download");
                    byte[] bytes = out.toByteArray();
                    verify(bytes, expectedHash);
                    return bytes;
                }
            } finally { connection.disconnect(); }
        }
        throw new IOException("Too many redirects");
    }

    static void verify(byte[] bytes) throws IOException {
        verify(bytes, SHA256);
    }

    static void verify(byte[] bytes, String expectedHash) throws IOException {
        try {
            byte[] digest = MessageDigest.getInstance("SHA-256").digest(bytes);
            StringBuilder hex = new StringBuilder();
            for (byte b : digest) hex.append(String.format(java.util.Locale.ROOT, "%02x", b & 255));
            if (!expectedHash.contentEquals(hex)) throw new IOException("Archive SHA-256 mismatch");
        } catch (NoSuchAlgorithmException e) { throw new IOException(e); }
    }

    static void unpack(InputStream archive, File staging) throws IOException {
        unpackZip(archive, staging, "Doukutsu/");
    }

    static void copyOverlay(InputStream archive, File staging) throws IOException {
        unpackZip(archive, staging, null);
    }

    static void prepareEnglish(byte[] archive, InputStream locale, File staging) throws IOException {
        prepareLanguage(archive, locale, staging, "en", "CaveStory/", ENGLISH_SOURCE, ENGLISH_SHA256, "english-install.json");
    }

    static void prepareJapanese(byte[] archive, InputStream locale, InputStream fonts, File staging) throws IOException {
        try (InputStream fontInput = fonts; InputStream localeInput = locale) {
            prepareLanguage(archive, localeInput, staging, "jp", "doukutsu/", JAPANESE_SOURCE, JAPANESE_SHA256, "japanese-install.json");
            copyOverlay(fontInput, staging);
        }
        if (!new File(staging, "fonts/japanese/chinese-12.fnt").isFile()) throw new IOException("Missing Japanese font");
    }

    private static void prepareLanguage(byte[] archive, InputStream locale, File staging, String language,
            String archiveRoot, String source, String hash, String marker) throws IOException {
        verify(archive, hash);
        checkCancelled();
        unpackZip(new ByteArrayInputStream(archive), staging, archiveRoot);
        File localeDir = new File(staging, "locale");
        if (!localeDir.mkdir()) throw new IOException("Cannot create English locale directory");
        try (InputStream input = locale; FileOutputStream output = new FileOutputStream(new File(localeDir, language + ".json"))) {
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) { checkCancelled(); output.write(buffer, 0, count); }
            output.getFD().sync();
        }
        for (String name : new String[]{"Doukutsu.exe", "Readme.txt", "Head.tsc", "ArmsItem.tsc", "Credit.tsc", "Stage/Start.tsc"}) {
            if (!new File(staging, name).isFile()) throw new IOException("Incomplete English data: " + name);
        }
        try (FileOutputStream output = new FileOutputStream(new File(staging, marker))) {
            output.write(("{\"version\":1,\"source\":\"" + source + "\",\"archive_sha256\":\"" + hash + "\"}\n").getBytes(StandardCharsets.UTF_8));
            output.getFD().sync();
        }
    }

    static boolean needsEnglish(File data) {
        return new File(data, "chinese-install.json").isFile() && !new File(data, "en").exists();
    }

    static boolean needsJapanese(File data) {
        return new File(data, "chinese-install.json").isFile() && !new File(data, "jp").exists();
    }

    static boolean needsJapanese(File data, InputStream fingerprints) throws IOException {
        return needsLanguage(data, fingerprints, "jp");
    }

    static boolean needsEnglish(File data, InputStream fingerprints) throws IOException {
        return needsLanguage(data, fingerprints, "en");
    }

    private static boolean needsLanguage(File data, InputStream fingerprints, String language) throws IOException {
        try (InputStream input = fingerprints) {
            if (new File(data, language).exists()) return false;
            if (new File(data, "chinese-install.json").isFile()) return true;
            // Earlier manually imported copies have no installation marker.
            // Recognize only the exact known base resources, never a modified game.
            if (!new File(data, "locale/zh-Hans.json").isFile()
                    || !new File(data, "fonts/chinese-12.fnt").isFile()) return false;
            java.util.Properties hashes = new java.util.Properties();
            hashes.load(input);
            if (hashes.size() != 400) throw new IOException("Invalid legacy resource fingerprints");
            for (String name : hashes.stringPropertyNames()) {
                File file = new File(data, name);
                if (!file.isFile() || file.length() > MAX_UNPACKED) return false;
                try (InputStream resource = new FileInputStream(file); ByteArrayOutputStream bytes = new ByteArrayOutputStream()) {
                    byte[] buffer = new byte[16384];
                    int count;
                    while ((count = resource.read(buffer)) != -1) {
                        if (bytes.size() + count > MAX_UNPACKED) return false;
                        bytes.write(buffer, 0, count);
                    }
                    try { verify(bytes.toByteArray(), hashes.getProperty(name)); }
                    catch (IOException mismatch) { return false; }
                }
            }
            return true;
        }
    }

    private static void unpackZip(InputStream archive, File staging, String gameRoot) throws IOException {
        if (!staging.isDirectory() && !staging.mkdirs()) throw new IOException("Cannot create staging directory");
        String root = staging.getCanonicalPath() + File.separator;
        Set<String> paths = new HashSet<>();
        long total = 0;
        int entries = 0;
        try (ZipInputStream zip = new ZipInputStream(archive, StandardCharsets.UTF_8)) {
            ZipEntry entry;
            byte[] buffer = new byte[16384];
            while ((entry = zip.getNextEntry()) != null) {
                checkCancelled();
                if (++entries > 1024) throw new IOException("Too many archive entries");
                String name = entry.getName();
                if (name.startsWith("/") || name.contains("\\") || name.contains(":") || name.indexOf('\0') >= 0)
                    throw new IOException("Unsafe archive path");
                for (String part : name.split("/")) {
                    if (part.equals("..") || part.equals(".")) throw new IOException("Unsafe archive path");
                }
                if (gameRoot != null) {
                    if (!name.startsWith(gameRoot)) throw new IOException("Unexpected archive root");
                    name = name.substring(gameRoot.length());
                    if (name.startsWith("data/")) name = name.substring(5);
                    else if (!(name.equals("Doukutsu.exe") || name.equals("Readme.txt") || name.equals("Manual.html") || name.startsWith("Manual/"))) {
                        // Do not import the legacy config, saves or configuration executable.
                        zip.closeEntry();
                        continue;
                    }
                }
                if (name.isEmpty()) continue;
                File target = new File(staging, name);
                if (!target.getCanonicalPath().startsWith(root) || !paths.add(name.toLowerCase(java.util.Locale.ROOT)))
                    throw new IOException("Duplicate or unsafe archive path");
                if (entry.isDirectory()) {
                    if (!target.isDirectory() && !target.mkdirs()) throw new IOException("Cannot create resource directory");
                } else {
                    if (!target.getParentFile().isDirectory() && !target.getParentFile().mkdirs()) throw new IOException("Cannot create resource directory");
                    if (!target.createNewFile()) throw new IOException("Refusing to replace a staged resource");
                    try (FileOutputStream out = new FileOutputStream(target)) {
                        int count;
                        while ((count = zip.read(buffer)) != -1) {
                            checkCancelled();
                            total += count;
                            if (total > MAX_UNPACKED) throw new IOException("Unpacked size limit exceeded");
                            out.write(buffer, 0, count);
                        }
                        out.getFD().sync();
                    }
                }
                zip.closeEntry();
            }
        }
    }

    static void validate(File staging) throws IOException {
        for (String name : new String[]{"Doukutsu.exe", "Readme.txt", "npc.tbl", "ArmsItem.tsc", "Head.tsc",
                "Credit.tsc", "Stage/Start.tsc", "Stage/Start.pxm", "bk0.pbm", "locale/zh-Hans.json",
                "locale/en.json", "locale/jp.json", "fonts/chinese-12.fnt", "fonts/licenses/OFL.txt",
                "chinese-install.json"}) {
            File file = new File(staging, name);
            if (!file.isFile() || file.length() == 0) throw new IOException("Missing resource: " + name);
        }
        for (int page = 0; page < 6; page++) {
            if (!new File(staging, "fonts/chinese-12_" + page + ".png").isFile()) throw new IOException("Missing font atlas");
        }
    }

    static void publish(File staging, File destination) throws IOException {
        checkCancelled();
        // A nonempty, inaccessible or non-directory destination is always user-owned.
        if (destination.exists()) {
            String[] files = destination.list();
            if (files == null || files.length != 0 || !destination.delete()) throw new IOException("Existing game data protected");
        }
        if (!staging.renameTo(destination)) throw new IOException("Cannot install prepared data");
    }

    // Caller holds the process-wide installer monitor: never remove a live attempt.
    static void removeAbandoned(File filesDir) throws IOException {
        filesDir = filesDir.getCanonicalFile(); // /data/user/0 may alias /data/data.
        File[] children = filesDir.listFiles();
        if (children == null) throw new IOException("Cannot inspect temporary directories");
        for (File child : children) {
            if (child.getName().matches("\\.chinese-install-[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}")
                    && child.isDirectory() && child.getCanonicalFile().equals(child.getAbsoluteFile())) {
                removeStaging(child);
            }
        }
    }

    /** Only call on the unique staging directory created by this download attempt. */
    static void removeStaging(File staging) throws IOException {
        staging = staging.getCanonicalFile();
        File[] files = staging.listFiles();
        if (files != null) for (File file : files) {
            if (file.isDirectory() && file.getCanonicalFile().equals(file.getAbsoluteFile())) removeStaging(file);
            else if (!file.delete()) throw new IOException("Cannot remove temporary file");
        }
        if (staging.exists() && !staging.delete()) throw new IOException("Cannot remove temporary directory");
    }
}
