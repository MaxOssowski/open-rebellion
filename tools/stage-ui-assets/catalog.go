package main

type dllTarget struct {
	Filename string
	// Aliases are other names an owned install ships the DLL under.
	Aliases         []string
	Directory       string
	Expected        int
	ExpectedType302 int
	// ExpectedRCData and ExpectedWave stage the advisor's clip records
	// (RCDATA/{id}.bin) and voice clips (WAVE/{id}.wav) when non-zero.
	ExpectedRCData int
	ExpectedWave   int
	// Optional targets are skipped when the source has none of their names.
	Optional bool
}

var uiDLLTargets = []dllTarget{
	{Filename: "COMMON.DLL", Directory: "common-dll", Expected: 321},
	{Filename: "GOKRES.DLL", Directory: "gokres-dll", Expected: 580},
	{Filename: "STRATEGY.DLL", Directory: "strategy-dll", Expected: 1042},
	{Filename: "TACTICAL.DLL", Directory: "tactical-dll", Expected: 288},
	{Filename: "ALSPRITE.DLL", Directory: "alsprite-dll", Expected: 38, ExpectedType302: 1640, ExpectedRCData: 752, ExpectedWave: 213},
	{Filename: "EMSPRITE.DLL", Directory: "emsprite-dll", Expected: 34, ExpectedType302: 2348, ExpectedRCData: 753, ExpectedWave: 216},
	// The opening briefing (FUN_0042d650, DLL slot 0xd). The GOG release
	// ships both renamed with an .OLD suffix.
	{Filename: "ALBRIEF.DLL", Aliases: []string{"ALBRIEF.DLL.OLD"}, Directory: "albrief-dll", Expected: 20, ExpectedType302: 2684, ExpectedRCData: 366, ExpectedWave: 17, Optional: true},
	{Filename: "EMBRIEF.DLL", Aliases: []string{"EMBRIEF.DLL.OLD"}, Directory: "embrief-dll", Expected: 18, ExpectedType302: 2738, ExpectedRCData: 471, ExpectedWave: 22, Optional: true},
	{Filename: "REBDLOG.DLL", Directory: "rebdlog-dll", Expected: 23},
}

// unloadedNamedBitmaps lists named bitmaps that REBEXE.EXE never loads by
// name. They are skipped instead of given an invented numeric ID.
var unloadedNamedBitmaps = map[string]bool{
	"DLG_CORNER_GRAB_FRAME": true,
}

var namedBitmapIDs = map[string]uint32{
	"COCKPIT_BUTTON_GAMESCALE_HUGE_UP":    15856,
	"COCKPIT_BUTTON_GAMESCALE_LARGE_UP":   15922,
	"COCKPIT_BUTTON_GAMESCALE_STD_UP":     15990,
	"DATA_BUTTON_UP_FIGHTERGROUP_RECOVER": 40720,
	"DATA_BUTTON_DN_FIGHTERGROUP_RECOVER": 40792,
	"DATA_BUTTON_UP_FIGHTERGROUP_TACTICS": 40864,
	"DATA_BUTTON_DN_FIGHTERGROUP_TACTICS": 40936,
}
