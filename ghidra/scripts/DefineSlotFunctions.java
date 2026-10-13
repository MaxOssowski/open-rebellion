// Define a function at every vtable slot target that has none.
//
// Ghidra's analysis leaves many small virtual methods undefined because
// nothing calls them directly. class-slots.tsv lists them (column 4).
//
// Arguments: class-slots.tsv
//@category OpenRebellion

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.TreeSet;

public class DefineSlotFunctions extends GhidraScript {
    @Override
    public void run() throws Exception {
        TreeSet<Long> targets = new TreeSet<>();
        for (String line : Files.readAllLines(new File(getScriptArgs()[0]).toPath(), StandardCharsets.UTF_8)) {
            String[] columns = line.split("\t", -1);
            if (columns.length >= 4 && columns[3].matches("[0-9a-fA-F]{8}")) {
                targets.add(Long.parseLong(columns[3], 16));
            }
        }
        int defined = 0;
        int inside = 0;
        int failed = 0;
        for (long target : targets) {
            monitor.checkCancelled();
            Address address = toAddr(target);
            if (getFunctionAt(address) != null) {
                continue;
            }
            Function containing = getFunctionContaining(address);
            if (containing != null) {
                // A slot pointing into the middle of a defined function.
                inside++;
                continue;
            }
            if (getInstructionAt(address) == null) {
                disassemble(address);
            }
            if (getInstructionAt(address) != null && createFunction(address, null) != null) {
                defined++;
            } else {
                failed++;
            }
        }
        println("slot targets: " + targets.size() + ", defined: " + defined
            + ", inside another function: " + inside + ", failed: " + failed);
    }
}
