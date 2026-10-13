// Label data addresses from a TSV whose first two columns are an address
// and a label (a header row is skipped). An address that already has a
// user-defined primary symbol is left alone.
//
// Arguments: LABELS.tsv
//@category OpenRebellion

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

public class ApplyLabels extends GhidraScript {
    @Override
    public void run() throws Exception {
        int labelled = 0;
        int kept = 0;
        for (String line : Files.readAllLines(new File(getScriptArgs()[0]).toPath(), StandardCharsets.UTF_8)) {
            String[] columns = line.split("\t", -1);
            if (columns.length < 2 || columns[1].isEmpty() || !columns[0].matches("[0-9a-fA-F]{6,8}")) {
                continue;
            }
            Address address = toAddr(columns[0]);
            Symbol primary = getSymbolAt(address);
            if (primary != null && primary.getSource() == SourceType.USER_DEFINED) {
                kept++;
                continue;
            }
            createLabel(address, columns[1], true, SourceType.USER_DEFINED);
            labelled++;
        }
        println("labelled: " + labelled + ", already labelled: " + kept);
    }
}
