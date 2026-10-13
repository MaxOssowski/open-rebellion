// Create a structure and a class namespace for every class in
// class-fields.tsv, move the class's methods (class-methods.tsv) into the
// namespace as __thiscall so `this` takes the structure, and optionally
// decompile those methods.
//
// Arguments: class-fields.tsv class-methods.tsv [OUTPUT_DIR]
//@category OpenRebellion

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.CategoryPath;
import ghidra.program.model.data.DataType;
import ghidra.program.model.data.DataTypeConflictHandler;
import ghidra.program.model.data.DataTypeManager;
import ghidra.program.model.data.EnumDataType;
import ghidra.program.model.data.IntegerDataType;
import ghidra.program.model.data.PointerDataType;
import ghidra.program.model.data.StructureDataType;
import ghidra.program.model.data.VoidDataType;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.GhidraClass;
import ghidra.program.model.symbol.Namespace;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.SymbolTable;
import ghidra.app.util.NamespaceUtils;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

public class ApplyClassTypes extends GhidraScript {
    private record Field(int offset, int size, String name, long mask, String declaredIn) {}

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        Map<String, List<Field>> fields = new LinkedHashMap<>();
        for (String line : Files.readAllLines(new File(args[0]).toPath(), StandardCharsets.UTF_8)) {
            String[] c = line.split("\t", -1);
            if (c.length < 7 || !c[2].startsWith("0x")) {
                continue;
            }
            fields.computeIfAbsent(c[0], key -> new ArrayList<>()).add(new Field(
                Integer.decode(c[2]), Integer.parseInt(c[3]), c[4],
                c[5].isEmpty() ? 0 : Long.decode(c[5]), c[6]));
        }

        DataTypeManager types = currentProgram.getDataTypeManager();
        SymbolTable symbols = currentProgram.getSymbolTable();
        Namespace global = currentProgram.getGlobalNamespace();
        DataType word = IntegerDataType.dataType;
        Map<String, GhidraClass> classes = new LinkedHashMap<>();

        for (Map.Entry<String, List<Field>> entry : fields.entrySet()) {
            String name = entry.getKey();
            // Flag bits by word, plain fields by offset.
            TreeMap<Integer, EnumDataType> flags = new TreeMap<>();
            TreeMap<Integer, Field> plain = new TreeMap<>();
            int size = 4;
            for (Field field : entry.getValue()) {
                size = Math.max(size, field.offset() + field.size());
                if (field.mask() != 0) {
                    EnumDataType bits = flags.computeIfAbsent(field.offset(), offset -> new EnumDataType(
                        CategoryPath.ROOT, field.declaredIn() + "_Flags" + Integer.toHexString(offset), 4, types));
                    if (!bits.contains(field.name())) {
                        bits.add(field.name(), field.mask());
                    }
                } else {
                    plain.put(field.offset(), field);
                }
            }
            StructureDataType structure = new StructureDataType(CategoryPath.ROOT, name, (size + 3) & ~3, types);
            structure.replaceAtOffset(0, new PointerDataType(VoidDataType.dataType), 4, "vtable", null);
            for (Field field : plain.values()) {
                if (field.offset() != 0 && !flags.containsKey(field.offset())) {
                    structure.replaceAtOffset(field.offset(), word, 4, field.name(),
                        "declared in " + field.declaredIn());
                }
            }
            for (Map.Entry<Integer, EnumDataType> bits : flags.entrySet()) {
                DataType stored = types.addDataType(bits.getValue(), DataTypeConflictHandler.REPLACE_HANDLER);
                structure.replaceAtOffset(bits.getKey(), stored, 4,
                    "Flags" + Integer.toHexString(bits.getKey()), null);
            }
            types.addDataType(structure, DataTypeConflictHandler.REPLACE_HANDLER);

            Namespace existing = symbols.getNamespace(name, global);
            GhidraClass owner;
            if (existing == null) {
                owner = symbols.createClass(global, name, SourceType.USER_DEFINED);
            } else if (existing instanceof GhidraClass) {
                owner = (GhidraClass) existing;
            } else {
                owner = NamespaceUtils.convertNamespaceToClass(existing);
            }
            classes.put(name, owner);
        }

        int moved = 0;
        int missing = 0;
        List<Function> methods = new ArrayList<>();
        for (String line : Files.readAllLines(new File(args[1]).toPath(), StandardCharsets.UTF_8)) {
            String[] c = line.split("\t", -1);
            if (c.length < 2 || !c[0].matches("[0-9a-fA-F]{8}") || !classes.containsKey(c[1])) {
                continue;
            }
            Function function = getFunctionAt(toAddr(c[0]));
            if (function == null) {
                missing++;
                continue;
            }
            function.setParentNamespace(classes.get(c[1]));
            function.setCallingConvention("__thiscall");
            methods.add(function);
            moved++;
        }
        println("classes: " + classes.size() + ", methods typed: " + moved + ", no function: " + missing);

        if (args.length > 2) {
            File outputDir = new File(args[2]);
            outputDir.mkdirs();
            DecompInterface decompiler = new DecompInterface();
            decompiler.openProgram(currentProgram);
            int written = 0;
            for (Function function : methods) {
                monitor.checkCancelled();
                DecompileResults results = decompiler.decompileFunction(function, 60, monitor);
                if (results != null && results.decompileCompleted()) {
                    File out = new File(outputDir, function.getParentNamespace().getName() + "__"
                        + function.getName() + "_" + function.getEntryPoint() + ".c");
                    Files.writeString(out.toPath(), results.getDecompiledFunction().getC(),
                        StandardCharsets.UTF_8);
                    written++;
                }
            }
            decompiler.dispose();
            println("decompiled: " + written);
        }
    }
}
