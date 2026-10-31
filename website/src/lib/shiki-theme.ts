type DocsPalette = {
  fg: string;
  comment: string;
  keyword: string;
  string: string;
  number: string;
  tag: string;
};

/** Minimal TextMate theme — soft pastel tokens for docs code panels. */
function docsTheme(name: string, type: "light" | "dark", colors: DocsPalette) {
  return {
    name,
    type,
    colors: {
      "editor.foreground": colors.fg,
      "editor.background": "#00000000",
    },
    tokenColors: [
      {
        scope: ["comment", "punctuation.definition.comment", "string.comment"],
        settings: { foreground: colors.comment, fontStyle: "italic" },
      },
      {
        scope: [
          "keyword",
          "storage",
          "storage.type",
          "storage.modifier",
          "keyword.control",
          "keyword.operator.new",
          "constant.language",
        ],
        settings: { foreground: colors.keyword },
      },
      {
        scope: [
          "string",
          "string.quoted",
          "string.template",
          "punctuation.definition.string",
        ],
        settings: { foreground: colors.string },
      },
      {
        scope: [
          "constant.numeric",
          "constant.language.boolean",
          "constant.other.option",
        ],
        settings: { foreground: colors.number },
      },
      {
        scope: [
          "entity.name.function",
          "support.function",
          "meta.function-call",
          "entity.name.macro",
          "support.macro",
          "entity.name.type",
          "entity.name.struct",
          "entity.name.enum",
          "entity.name.class",
          "support.type",
          "support.class",
          "entity.name.tag",
          "support.type.property-name",
          "meta.object-literal.key",
        ],
        settings: { foreground: colors.tag },
      },
      {
        scope: [
          "constant.other.placeholder",
          "variable.language",
          "variable.other.constant",
          "support.variable",
        ],
        settings: { foreground: colors.number },
      },
      {
        scope: ["markup.heading", "entity.name.section"],
        settings: { foreground: colors.keyword },
      },
    ],
  };
}

/** Soft pastel tokens for KUI docs chrome (not GitHub theme). */
export const kuiDocsLight = docsTheme("kui-docs-light", "light", {
  fg: "#171717",
  comment: "#737373",
  keyword: "oklch(50% 0.16 330)",
  string: "oklch(48% 0.13 160)",
  number: "oklch(53% 0.14 55)",
  tag: "oklch(50% 0.15 255)",
});

export const kuiDocsDark = docsTheme("kui-docs-dark", "dark", {
  fg: "#f5f5f5",
  comment: "#8a8a8a",
  keyword: "oklch(78% 0.13 330)",
  string: "oklch(80% 0.12 160)",
  number: "oklch(82% 0.12 70)",
  tag: "oklch(78% 0.12 250)",
});
