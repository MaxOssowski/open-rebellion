// Give lookup functions a class pointer return type so the decompiler
// carries the class structure into their callers, then optionally
// decompile every function that is not an unwind funclet.
//
// Arguments: lookup-types.tsv [OUTPUT_DIR]
//@category OpenRebellion

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.CategoryPath;
import ghidra.program.model.data.DataType;
import ghidra.program.model.data.PointerDataType;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

public class SetReturnTypes extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        int typed = 0;
        int skipped = 0;
        for (String line : Files.readAllLines(new File(args[0]).toPath(), StandardCharsets.UTF_8)) {
            String[] columns = line.split("\t", -1);
            if (columns.length < 2 || !columns[0].matches("[0-9a-fA-F]{8}")) {
                continue;
            }
            Function function = getFunctionAt(toAddr(columns[0]));
            DataType structure = currentProgram.getDataTypeManager()
                .getDataType(CategoryPath.ROOT, columns[1]);
            if (function == null || structure == null) {
                skipped++;
                continue;
            }
            function.setReturnType(new PointerDataType(structure), SourceType.USER_DEFINED);
            typed++;
        }
        println("return types set: " + typed + ", skipped: " + skipped);

        if (args.length > 1) {
            File outputDir = new File(args[1]);
            outputDir.mkdirs();
            DecompInterface decompiler = new DecompInterface();
            decompiler.openProgram(currentProgram);
            int written = 0;
            for (Function function : currentProgram.getFunctionManager().getFunctions(true)) {
                monitor.checkCancelled();
                if (function.isThunk() || function.isExternal() || function.getName().startsWith("Unwind@")) {
                    continue;
                }
                DecompileResults results = decompiler.decompileFunction(function, 60, monitor);
                if (results != null && results.decompileCompleted()) {
                    File out = new File(outputDir, "FUN_" + function.getEntryPoint() + ".c");
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
