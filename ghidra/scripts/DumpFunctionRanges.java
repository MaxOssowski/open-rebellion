// Write each function's entry and the first and last address of its body.
//
// Arguments: OUTPUT.tsv
//@category OpenRebellion

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.AddressRange;
import ghidra.program.model.listing.Function;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.List;

public class DumpFunctionRanges extends GhidraScript {
    @Override
    public void run() throws Exception {
        List<String> rows = new ArrayList<>();
        rows.add("function\tname\tfirst\tlast");
        for (Function function : currentProgram.getFunctionManager().getFunctions(true)) {
            for (AddressRange range : function.getBody()) {
                rows.add(function.getEntryPoint() + "\t" + function.getName() + "\t"
                    + range.getMinAddress() + "\t" + range.getMaxAddress());
            }
        }
        Files.write(new File(getScriptArgs()[0]).toPath(), rows, StandardCharsets.UTF_8);
        println("ranges: " + (rows.size() - 1));
    }
}
