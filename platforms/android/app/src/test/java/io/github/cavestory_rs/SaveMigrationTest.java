package io.github.cavestory_rs;
import java.io.*;
import java.nio.file.*;
import java.util.*;

public final class SaveMigrationTest {
    static void check(boolean ok, String why) { if (!ok) throw new AssertionError(why); }
    static class Disk implements SaveMigration.Store {
        final Path root; boolean fail, corrupt;
        Disk(Path root) throws IOException { this.root=Files.createDirectories(root); }
        public List<String> names() throws IOException {
            try (java.util.stream.Stream<Path> files=Files.list(root)) {
                return files.map(p->p.getFileName().toString()).collect(java.util.stream.Collectors.toList());
            }
        }
        public byte[] read(String path) throws IOException { return Files.exists(root.resolve(path))?Files.readAllBytes(root.resolve(path)):null; }
        public void create(String path, byte[] bytes) throws IOException {
            if(fail) { Files.write(root.resolve(path),new byte[]{1},StandardOpenOption.CREATE_NEW); throw new IOException("disk full"); }
            Files.write(root.resolve(path),corrupt?new byte[]{2}:bytes,StandardOpenOption.CREATE_NEW);
        }
        public void rename(String from,String to) throws IOException { Files.move(root.resolve(from),root.resolve(to)); }
        public void delete(String path) throws IOException { Files.deleteIfExists(root.resolve(path)); }
        void put(String path,String value) throws IOException { Files.write(root.resolve(path),value.getBytes()); }
        String text(String path) throws IOException { byte[] b=read(path); return b==null?null:new String(b); }
    }
    public static void main(String[] args) throws Exception {
        Path base=Files.createTempDirectory(Path.of(args[0]),"migration-");
        Disk from=new Disk(base.resolve("source")), to=new Disk(base.resolve("target"));
        File recovery=base.resolve("recovery").toFile();
        from.put("Profile.dat","early"); from.put("Profile2.dat","later"); from.put("mod_req.json","progress");
        from.put("290.rec","timer"); from.put("290.last.rep","replay"); from.put("settings.json","device");
        from.put("image.png","resource");
        SaveMigration.copy(from,to,recovery,"first");
        check("early".equals(to.text("Profile.dat")),"copy must publish source save");
        check("later".equals(to.text("Profile2.dat")),"all slots copied");
        check(to.read("settings.json")==null && to.read("image.png")==null,"device settings and images stay private");
        check("timer".equals(to.text("290.rec")) && "replay".equals(to.text("290.last.rep")),"timer and replay copied");
        check("early".equals(from.text("Profile.dat")),"source retained");
        SaveMigration.copy(from,to,recovery,"first");
        to.put("Profile.dat","foreign");
        check(SaveMigration.conflicts(from,to).equals(Arrays.asList("Profile.dat")),"identify conflict before any copy");
        from.put("Profile3.dat","third");
        try { SaveMigration.copy(from,to,recovery,"conflict"); throw new AssertionError("conflict accepted"); } catch(IOException expected) { }
        check("foreign".equals(to.text("Profile.dat")) && to.read("Profile3.dat")==null,"conflict must not overwrite or partially merge");
        Disk interrupted=new Disk(base.resolve("interrupted")); interrupted.fail=true;
        try { SaveMigration.copy(from,interrupted,recovery,"resume"); throw new AssertionError("failure accepted"); } catch(IOException expected) { }
        check(interrupted.read("Profile.dat")==null,"partial file must not be published");
        interrupted.fail=false;
        SaveMigration.copy(from,interrupted,recovery,"resume");
        check("third".equals(interrupted.text("Profile3.dat")),"retry after partial write");
        Disk corrupt=new Disk(base.resolve("corrupt")); corrupt.corrupt=true;
        try { SaveMigration.copy(from,corrupt,recovery,"corrupt"); throw new AssertionError("bad checksum accepted"); } catch(IOException expected) { }
        check(corrupt.read("Profile.dat")==null,"readback before publication");
        Disk race=new Disk(base.resolve("race")) {
            @Override public void rename(String a,String b) throws IOException { put(b,"outside edit"); super.rename(a,b); }
        };
        try { SaveMigration.copy(from,race,recovery,"race"); throw new AssertionError("race accepted"); } catch(IOException expected) { }
        check("outside edit".equals(race.text("290.last.rep")),"late destination edits preserved");
        System.out.println("SaveMigrationTest: copy, filtering, retention, same-content, conflict, interruption, checksum, race passed");
    }
}
