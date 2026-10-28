package main

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
)

type verifySummary struct {
	DLLs      int
	Resources int
}

func verifyTargets(outputDir string, targets []dllTarget, stdout io.Writer) (verifySummary, error) {
	summary := verifySummary{}
	for _, target := range targets {
		if target.Optional {
			if _, err := os.Stat(filepath.Join(outputDir, target.Directory)); os.IsNotExist(err) {
				fmt.Fprintf(stdout, "%s: not staged, skipped\n", target.Directory)
				continue
			}
		}
		bmpDir := filepath.Join(outputDir, target.Directory, "BMP")
		entries, err := os.ReadDir(bmpDir)
		if err != nil {
			return summary, fmt.Errorf("%s: read staged directory: %w", target.Directory, err)
		}

		count := 0
		for _, entry := range entries {
			if entry.IsDir() || filepath.Ext(entry.Name()) != ".bmp" {
				continue
			}
			var resourceID uint32
			if _, err := fmt.Sscanf(entry.Name(), "%d.bmp", &resourceID); err != nil || entry.Name() != fmt.Sprintf("%d.bmp", resourceID) {
				return summary, fmt.Errorf("%s: non-numeric bitmap filename %q", target.Directory, entry.Name())
			}
			bmp, err := os.ReadFile(filepath.Join(bmpDir, entry.Name()))
			if err != nil {
				return summary, fmt.Errorf("%s/%s: %w", target.Directory, entry.Name(), err)
			}
			if err := validateBMP(bmp); err != nil {
				return summary, fmt.Errorf("%s/%s: %w", target.Directory, entry.Name(), err)
			}
			count++
		}
		if count != target.Expected {
			return summary, fmt.Errorf("%s: found %d staged BMPs, expected %d", target.Directory, count, target.Expected)
		}
		summary.DLLs++
		summary.Resources += count
		fmt.Fprintf(stdout, "%s: verified %d BMPs\n", target.Directory, count)

		if target.ExpectedType302 > 0 {
			frameDir := filepath.Join(outputDir, target.Directory, "TYPE302")
			entries, err := os.ReadDir(frameDir)
			if err != nil {
				return summary, fmt.Errorf("%s: read staged type-302 directory: %w", target.Directory, err)
			}
			frameCount := 0
			for _, entry := range entries {
				if entry.IsDir() || filepath.Ext(entry.Name()) != ".bin" {
					continue
				}
				var resourceID uint32
				if _, err := fmt.Sscanf(entry.Name(), "%d.bin", &resourceID); err != nil || entry.Name() != fmt.Sprintf("%d.bin", resourceID) {
					return summary, fmt.Errorf("%s: non-numeric type-302 filename %q", target.Directory, entry.Name())
				}
				data, err := os.ReadFile(filepath.Join(frameDir, entry.Name()))
				if err != nil {
					return summary, fmt.Errorf("%s/%s: %w", target.Directory, entry.Name(), err)
				}
				if err := validateType302(data); err != nil {
					return summary, fmt.Errorf("%s/%s: %w", target.Directory, entry.Name(), err)
				}
				frameCount++
			}
			if frameCount != target.ExpectedType302 {
				return summary, fmt.Errorf("%s: found %d staged type-302 frames, expected %d", target.Directory, frameCount, target.ExpectedType302)
			}
			summary.Resources += frameCount
			fmt.Fprintf(stdout, "%s: verified %d type-302 frames\n", target.Directory, frameCount)
		}
		for _, staged := range []struct {
			expected        int
			kind, extension string
			validate        func([]byte) error
		}{
			{target.ExpectedRCData, "RCDATA", "bin", func([]byte) error { return nil }},
			{target.ExpectedWave, "WAVE", "wav", validateWAV},
		} {
			if staged.expected == 0 {
				continue
			}
			count, err := countStaged(filepath.Join(outputDir, target.Directory, staged.kind), staged.extension, staged.validate)
			if err != nil {
				return summary, fmt.Errorf("%s/%s: %w", target.Directory, staged.kind, err)
			}
			if count != staged.expected {
				return summary, fmt.Errorf("%s: found %d staged %s resources, expected %d", target.Directory, count, staged.kind, staged.expected)
			}
			summary.Resources += count
			fmt.Fprintf(stdout, "%s: verified %d %s resources\n", target.Directory, count, staged.kind)
		}
	}
	return summary, nil
}

// countStaged counts the {id}.{extension} files in dir, each passing validate.
func countStaged(dir, extension string, validate func([]byte) error) (int, error) {
	entries, err := os.ReadDir(dir)
	if err != nil {
		return 0, err
	}
	count := 0
	for _, entry := range entries {
		if entry.IsDir() || filepath.Ext(entry.Name()) != "."+extension {
			continue
		}
		var resourceID uint32
		if _, err := fmt.Sscanf(entry.Name(), "%d."+extension, &resourceID); err != nil || entry.Name() != fmt.Sprintf("%d.%s", resourceID, extension) {
			return count, fmt.Errorf("non-numeric filename %q", entry.Name())
		}
		data, err := os.ReadFile(filepath.Join(dir, entry.Name()))
		if err != nil {
			return count, err
		}
		if err := validate(data); err != nil {
			return count, fmt.Errorf("%s: %w", entry.Name(), err)
		}
		count++
	}
	return count, nil
}
