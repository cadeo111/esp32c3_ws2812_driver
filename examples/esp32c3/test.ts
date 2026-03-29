// scripts/analyze-stack.ts
import { execSync } from "child_process";
import * as fs from "fs";

interface StackEntry {
    mangledName: string;
    demangledName: string;
    size: number;
    file: string | null;
    line: number | null;
}

function parseStackSizes(output: string): Map<string, number> {
    const map = new Map<string, number>();
    const entryRegex =
        /Entry \{\s+Functions: \[([^\]]+)\]\s+Size: (0x[0-9a-fA-F]+)/g;

    let match;
    while ((match = entryRegex.exec(output)) !== null) {
        const names = match[1].split(",").map((s) => s.trim());
        const size = parseInt(match[2], 16);
        for (const name of names) {
            map.set(name, size);
        }
    }
    return map;
}

function getSymbolAddresses(binaryPath: string): Map<string, string> {
    const map = new Map<string, string>();
    const output = execSync(
        `llvm-nm --defined-only --format=posix ${binaryPath}`
    ).toString();

    for (const line of output.split("\n")) {
        const parts = line.trim().split(/\s+/);
        if (parts.length >= 3) {
            const [name, , address] = parts;
            map.set(name, address);
        }
    }
    return map;
}

function getSourceLocation(
    binaryPath: string,
    address: string
): { file: string; line: number } | null {
    try {
        const output = execSync(
            `llvm-addr2line -f -C -e ${binaryPath} 0x${address}`
        )
            .toString()
            .trim();

        const lines = output.split("\n");
        if (lines.length >= 2) {
            const [fileLine] = lines.slice(1);
            const lastColon = fileLine.lastIndexOf(":");
            const file = fileLine.substring(0, lastColon);
            const line = parseInt(fileLine.substring(lastColon + 1));

            if (!isNaN(line) && file !== "??") {
                return { file, line };
            }
        }
    } catch { }
    return null;
}

function demangleSymbol(name: string): string {
    try {
        return execSync(`echo "${name}" | rustfilt`).toString().trim();
    } catch {
        return name;
    }
}

export function analyze(binaryPath: string): StackEntry[] {
    const rawOutput = execSync(
        `llvm-readobj --stack-sizes ${binaryPath}`
    ).toString();

    const stackSizes = parseStackSizes(rawOutput);
    const addresses = getSymbolAddresses(binaryPath);
    const results: StackEntry[] = [];

    for (const [mangled, size] of stackSizes) {
        const address = addresses.get(mangled);
        const location = address
            ? getSourceLocation(binaryPath, address)
            : null;

        results.push({
            mangledName: mangled,
            demangledName: demangleSymbol(mangled),
            size,
            file: location?.file ?? null,
            line: location?.line ?? null,
        });
    }

    return results;
}

// Output JSON for the extension to consume
const binaryPath = process.argv[2];
const results = analyze(binaryPath);
fs.writeFileSync(
    "stack-sizes.json",
    JSON.stringify(results, null, 2)
);