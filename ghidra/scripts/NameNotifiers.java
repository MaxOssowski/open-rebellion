// Name each notifier function from the "...Notif" string it passes to the
// notify helpers, and list the vtable slots that point at it.
//
// Arguments: OUTPUT.tsv [dry-run]
// Columns: function, name, field, vtable, slot.
//@category OpenRebellion

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.SourceType;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class NameNotifiers extends GhidraScript {
    private static final Pattern NOTIF = Pattern.compile("[A-Za-z][A-Za-z0-9]*Notif");
    private static final Pattern FIELD = Pattern.compile("[A-Za-z][A-Za-z0-9]*");
    /** How far back from a slot a vtable's first entry may lie. */
    private static final int MAX_VTABLE_BYTES = 0x400;

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        File output = new File(args[0]);
        boolean dryRun = args.length > 1 && args[1].equals("dry-run");

        // Function -> the Notif names and other strings it references.
        Map<Function, Set<String>> names = new TreeMap<>((a, b) -> a.getEntryPoint().compareTo(b.getEntryPoint()));
        Map<Function, Set<String>> fields = new LinkedHashMap<>();
        for (Data data : currentProgram.getListing().getDefinedData(true)) {
            monitor.checkCancelled();
            if (!data.hasStringValue()) {
                continue;
            }
            String text = String.valueOf(data.getValue());
            Matcher notif = NOTIF.matcher(text);
            boolean isNotif = notif.find();
            for (Reference reference : getReferencesTo(data.getAddress())) {
                Function function = getFunctionContaining(reference.getFromAddress());
                if (function == null) {
                    continue;
                }
                if (isNotif) {
                    names.computeIfAbsent(function, key -> new LinkedHashSet<>()).add(notif.group());
                } else if (FIELD.matcher(text.trim()).matches()) {
                    fields.computeIfAbsent(function, key -> new LinkedHashSet<>()).add(text.trim());
                }
            }
        }

        Map<String, Integer> used = new LinkedHashMap<>();
        List<String> rows = new ArrayList<>();
        rows.add("function\tname\tfield\tvtable\tslot");
        int renamed = 0;
        int ambiguous = 0;
        for (Map.Entry<Function, Set<String>> entry : names.entrySet()) {
            Function function = entry.getKey();
            if (entry.getValue().size() != 1) {
                ambiguous++;
                rows.add(function.getEntryPoint() + "\t?" + String.join("|", entry.getValue()) + "\t\t\t");
                continue;
            }
            String base = entry.getValue().iterator().next();
            int count = used.merge(base, 1, Integer::sum);
            String name = count == 1 ? base : base + "_" + function.getEntryPoint();
            Set<String> field = fields.getOrDefault(function, Set.of());
            String fieldName = field.size() == 1 ? field.iterator().next() : "";
            if (!dryRun && function.getSymbol().getSource() == SourceType.DEFAULT) {
                function.setName(name, SourceType.USER_DEFINED);
                renamed++;
            }
            List<String> slots = vtableSlots(function);
            if (slots.isEmpty()) {
                rows.add(function.getEntryPoint() + "\t" + name + "\t" + fieldName + "\t\t");
            }
            for (String slot : slots) {
                rows.add(function.getEntryPoint() + "\t" + name + "\t" + fieldName + "\t" + slot);
            }
        }
        Files.write(output.toPath(), rows, StandardCharsets.UTF_8);
        println("notifier functions: " + names.size() + ", ambiguous: " + ambiguous
            + ", renamed: " + renamed + ", rows: " + (rows.size() - 1));
    }

    /** "vtable\tslot" for every data pointer to the function's entry. */
    private List<String> vtableSlots(Function function) throws Exception {
        List<String> slots = new ArrayList<>();
        for (Reference reference : getReferencesTo(function.getEntryPoint())) {
            Address from = reference.getFromAddress();
            if (getInstructionContaining(from) != null) {
                continue;
            }
            MemoryBlock block = getMemoryBlock(from);
            if (block == null || !block.isInitialized()) {
                continue;
            }
            Address start = vtableStart(from, block);
            if (start != null) {
                slots.add(start + "\t0x" + Long.toHexString(from.subtract(start)));
            }
        }
        return slots;
    }

    /**
     * The nearest address at or before `slot` that an instruction refers
     * to (a constructor storing the vtable pointer), reached through
     * pointers to functions only.
     */
    private Address vtableStart(Address slot, MemoryBlock block) throws Exception {
        Address at = slot;
        for (int walked = 0; walked <= MAX_VTABLE_BYTES; walked += 4) {
            if (!block.contains(at)) {
                return null;
            }
            Address target = toAddr(getInt(at) & 0xffffffffL);
            if (getFunctionAt(target) == null && getInstructionAt(target) == null) {
                return null;
            }
            for (Reference reference : getReferencesTo(at)) {
                if (getInstructionContaining(reference.getFromAddress()) != null) {
                    return at;
                }
            }
            at = at.subtract(4);
        }
        return null;
    }
}
