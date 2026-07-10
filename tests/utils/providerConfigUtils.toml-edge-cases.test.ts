import { describe, expect, it } from "vitest";
import { parse as parseToml } from "smol-toml";
import {
  extractCodexBaseUrl,
  extractCodexModelName,
  extractCodexTopLevelInt,
  setCodexBaseUrl,
  setCodexGoalMode,
  setCodexModelName,
  setCodexTopLevelInt,
  isCodexGoalModeEnabled,
} from "@/utils/providerConfigUtils";

/**
 * M13 characterization tests.
 *
 * These pin the CURRENT behavior of the hand-rolled Codex `config.toml` line
 * editors — including their known imperfections — so a future refactor (e.g. a
 * naive swap to `smol-toml` round-tripping) fails loudly instead of silently
 * regressing comment/format preservation. See the design note at the top of the
 * "TOML Config Utilities" section in providerConfigUtils.ts.
 */
describe("providerConfigUtils TOML edge cases (characterization)", () => {
  describe("trailing inline comments are preserved on edit (B6g)", () => {
    it("setCodexBaseUrl preserves a trailing inline comment on the rewritten line", () => {
      const input = 'base_url = "https://old/v1" # keep me\nmodel = "m"\n';
      expect(setCodexBaseUrl(input, "https://new/v1")).toBe(
        'base_url = "https://new/v1" # keep me\nmodel = "m"\n',
      );
    });

    it("setCodexModelName preserves a trailing inline comment on the rewritten line", () => {
      expect(setCodexModelName('model = "old" # c\n', "new")).toBe(
        'model = "new" # c\n',
      );
    });

    it("setCodexTopLevelInt preserves a trailing inline comment on the rewritten line", () => {
      expect(
        setCodexTopLevelInt(
          "request_max_retries = 3 # tune\n",
          "request_max_retries",
          5,
        ),
      ).toBe("request_max_retries = 5 # tune\n");
    });

    it("setCodexBaseUrl keeps a # inside the value AND a real trailing comment", () => {
      // The in-quote `#frag` must NOT be treated as a comment, while the genuine
      // trailing `# real comment` must be preserved.
      expect(
        setCodexBaseUrl(
          'base_url = "https://x/v1?a=#frag" # real comment\n',
          "https://y/v1?b=#z",
        ),
      ).toBe('base_url = "https://y/v1?b=#z" # real comment\n');
    });
  });

  describe("hash (#) inside a quoted value is handled correctly", () => {
    it("extractCodexBaseUrl keeps a fragment-like # inside the quoted value", () => {
      expect(extractCodexBaseUrl('base_url = "https://x/v1?a=#frag"\n')).toBe(
        "https://x/v1?a=#frag",
      );
    });

    it("setCodexBaseUrl preserves a # inside the new quoted value", () => {
      expect(
        setCodexBaseUrl(
          'base_url = "https://x/v1?a=#frag"\n',
          "https://y/v1#z",
        ),
      ).toBe('base_url = "https://y/v1#z"\n');
    });
  });

  describe("KNOWN LIMITATION: inline-table providers are not recognized", () => {
    const inlineTable =
      'model_provider = "custom"\n' +
      'model_providers = { custom = { base_url = "https://inline/v1", name = "X" } }\n';

    it("extractCodexBaseUrl cannot read base_url from an inline-table provider", () => {
      expect(extractCodexBaseUrl(inlineTable)).toBeUndefined();
    });

    it("setCodexBaseUrl appends a duplicate [model_providers.custom] table → invalid TOML", () => {
      const result = setCodexBaseUrl(inlineTable, "https://new/v1");
      // The original inline table is left untouched...
      expect(result).toContain(
        'model_providers = { custom = { base_url = "https://inline/v1", name = "X" } }',
      );
      // ...and a second, conflicting table is appended.
      expect(result).toContain(
        '[model_providers.custom]\nbase_url = "https://new/v1"',
      );
      // The duplicate key makes the result unparseable (documents the corruption).
      expect(() => parseToml(result)).toThrow();
    });
  });

  describe("KNOWN LIMITATION: multiline strings and dotted keys not handled", () => {
    it("extractCodexModelName returns undefined for a multiline basic string", () => {
      const input = 'model = """\ngpt-5\n"""\nbase_url = "https://x/v1"\n';
      expect(extractCodexModelName(input)).toBeUndefined();
      // base_url on its own line is still recovered by the line scanner.
      expect(extractCodexBaseUrl(input)).toBe("https://x/v1");
    });

    it("extractCodexBaseUrl returns undefined for a dotted key inside a table", () => {
      const input =
        'model_provider = "custom"\n\n[model_providers]\ncustom.base_url = "https://dotted/v1"\n';
      expect(extractCodexBaseUrl(input)).toBeUndefined();
    });

    it("extractCodexTopLevelInt reads a simple top-level integer", () => {
      expect(
        extractCodexTopLevelInt(
          "request_max_retries = 3\n",
          "request_max_retries",
        ),
      ).toBe(3);
    });
  });

  describe("KNOWN LIMITATION: CRLF input yields mixed line endings", () => {
    it("setCodexGoalMode inserts LF-delimited [features] amid CRLF lines", () => {
      const input =
        'model = "m"\r\n[model_providers.custom]\r\nname = "custom"\r\n';
      const output = setCodexGoalMode(input, true);

      expect(isCodexGoalModeEnabled(output)).toBe(true);
      expect(output).toContain("[features]\ngoals = true");
      // Pre-existing CRLF lines are preserved verbatim (mixed endings) — the
      // result still parses, but the layout is inconsistent.
      expect(output).toContain("\r\n");
      expect(() => parseToml(output)).not.toThrow();
    });
  });
});
