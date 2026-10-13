// Rename functions from a TSV whose first two columns are an address and a
// name (a header row and rows with an empty name are skipped). Functions
// that already carry a non-default name are left alone, except that a
// generated EventNNN_/TimerNNN_ name may be replaced by a fuller one.
//
// Arguments: NAMES.tsv
//@category OpenRebellion

import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

public class ApplyNames extends GhidraScript {
    /** Both names carry the same generated "Event300" or "Timer382" tag. */
    private static boolean sameGeneratedTag(String current, String wanted) {
        String tag = wanted.split("_", 2)[0];
        return tag.matches("(Event|Timer)[0-9A-F]{3}") && current.split("_", 2)[0].equals(tag);
    }

    @Override
    public void run() throws Exception {
        int renamed = 0;
        int kept = 0;
        int missing = 0;
        for (String line : Files.readAllLines(new File(getScriptArgs()[0]).toPath(), StandardCharsets.UTF_8)) {
            String[] columns = line.split("\t", -1);
            if (columns.length < 2 || columns[1].isEmpty() || !columns[0].matches("[0-9a-fA-F]{6,8}")) {
                continue;
            }
            Function function = getFunctionAt(toAddr(columns[0]));
            if (function == null) {
                missing++;
            } else if (function.getSymbol().getSource() != SourceType.DEFAULT
                && !sameGeneratedTag(function.getName(), columns[1])) {
                kept++;
            } else if (function.getName().equals(columns[1])) {
                kept++;
            } else {
                function.setName(columns[1], SourceType.USER_DEFINED);
                renamed++;
            }
        }
        println("renamed: " + renamed + ", already named: " + kept + ", no function: " + missing);
    }
}
