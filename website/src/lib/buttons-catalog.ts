import workspace from "./workspace-docs.json";
import fields from "./text-field-docs.json";
import collections from "./collection-docs.json";
import family from "./family-docs.json";
/** Buttons family catalog — docs metadata for the website. Includes installable Rust stories and contract previews. */

export type ButtonDocId =
  | "button"
  | "action-button"
  | "split-button"
  | "button-group"
  | "floating-button-group"
  | "expanding-button-group"
  | "copy-button"
  | "action-swap"
  | "icon-morph"
  | "confirm-morph"
  | "theme-switch"
  | "hold-to-confirm" | "swipe-actions" | "reactions" | "dropdown-menu" | "context-menu" | "user-menu" | "input" | "textarea" | "password-field" | "password-strength" | "search-field" | "expanding-search" | "inline-edit" | "dialog" | "popover" | "tabs" | "combobox" | "select" | "switch" | "resizable-panels" | "empty-state";

export type Locale = "zh" | "en";

export type PropRow = { name: string; type: string; default?: string; description: string };

export type ButtonDoc = {
  id: ButtonDocId;
  /** Localized nav / page title. */
  title: Record<Locale, string>;
  group?: "buttons" | "gestures" | "menus" | "text-fields" | "overlays" | "navigation" | "selects" | "toggles" | "expand" | "cards";
  installable: boolean;
  story?: boolean;
  keyboard?: Record<Locale,string>;
  rustType: string; // e.g. ActionButton
  usageRust: string;
  installShell: string;
  copy: Record<
    Locale,
    {
      lead: string;
      when: string[];
      not: string[];
      apiLead: string;
      motion: string[];
      ai: string[];
      props: PropRow[];
    }
  >;
};

const row = (name: string, type: string, description: string, def?: string): PropRow =>
  def === undefined ? { name, type, description } : { name, type, default: def, description };

export const BUTTONS_CATALOG: ButtonDoc[] = [
  {
    id: "button",
    story:true,keyboard:{en:"Tab to focus; press Enter or Space to activate.",zh:"Tab 聚焦，Enter 或空格激活。"},
    title: { en: "Button", zh: "按钮" },
    installable: true,
    rustType: "Button",
    usageRust: `use kui::components::button::Button;

Button::new("save")
    .primary()
    .label("Save")
    .into_themed(cx);`,
    installShell: `cargo install --path tools/cli
kui init
kui add button`,
    copy: {
      en: {
        lead: "Small, medium and large buttons use the host theme’s radius.md (8px by default). The medium button is 44px high.",
        when: [
          "Primary, secondary, ghost, and destructive actions in a KUI app.",
          "Any single action that runs on click.",
        ],
        not: [
          "Navigation that should be a link.",
          "A parallel button when this item already fits.",
        ],
        apiLead: "Chainable builders that consume self. Call into_themed(cx) to apply live KUI tokens.",
        motion: [
          "Press feedback uses MOTION.snappy (about 0.97 scale).",
          "Collapses to no scale when Theme.reduced_motion is true.",
          "Primary disabled uses 52% opacity; other variants keep shape and mute the label.",
        ],
        ai: [
          "Prefer `kui add button` over rewriting the control.",
          "Customize through theme tokens and public builders.",
          "Keep ElementId stable; changing it resets focus.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id; part of the public contract."),
          row("label", "impl Into<SharedString>", "Visible text and accessible name.", '""'),
          row("primary / secondary / ghost / danger", "ButtonVariant", "Selects the variant.", "primary"),
          row("with_size", "Size · Sm | Md | Lg", "Control height and padding.", "Md"),
          row("loading", "bool", "Shows a spinner; keeps keyboard focus and blocks activation.", "false"),
          row("disabled", "bool", "Blocks clicks.", "false"),
          row("on_click", "Fn(&ClickEvent, &mut Window, &mut App)", "Click handler."),
          row("into_themed(cx)", "&App → Stateful<Div>", "Applies KUI theme tokens."),
        ],
      },
      zh: {
        lead: "小、中、大三种尺寸都使用宿主主题的 radius.md，默认圆角为 8px。中号按钮高 44px。",
        when: ["KUI 应用中的主要、次要、幽灵与破坏性操作。", "任何点击即执行的单一操作。"],
        not: ["应当作为链接的导航。", "已有此组件时，不要另造一个并行按钮。"],
        apiLead: "链式构建器，按值消费 self。调用 into_themed(cx) 应用实时 KUI 令牌。",
        motion: [
          "按压反馈使用 MOTION.snappy（约 0.97 缩放）。",
          "Theme.reduced_motion 为 true 时取消缩放。",
          "主要按钮禁用时为 52% 不透明度；其他变体保持形状并弱化文字。",
        ],
        ai: [
          "优先使用 `kui add button`，不要重写该控件。",
          "通过主题令牌和公开构建器定制。",
          "保持 ElementId 稳定；更改会重置焦点。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id，属于公开契约。"),
          row("label", "impl Into<SharedString>", "可见文字，同时作为可访问名称。", '""'),
          row("primary / secondary / ghost / danger", "ButtonVariant", "选择变体。", "primary"),
          row("with_size", "Size · Sm | Md | Lg", "控件高度与内边距。", "Md"),
          row("loading", "bool", "显示加载指示器、保留焦点并阻止重复激活。", "false"),
          row("disabled", "bool", "阻止点击。", "false"),
          row("on_click", "Fn(&ClickEvent, &mut Window, &mut App)", "点击处理函数。"),
          row("into_themed(cx)", "&App → Stateful<Div>", "应用 KUI 主题令牌。"),
        ],
      },
    },
  },
  ...family.map(entry => {
    const segmented = ["split-button", "button-group", "floating-button-group", "expanding-button-group", "confirm-morph"].includes(entry.id);
    const shape = segmented
      ? { en: "The outer control uses radius.md (8px by default). Nested controls and highlights use radius.sm (4px by default).", zh: "外框使用 radius.md（默认 8px），内嵌控件与高亮使用 radius.sm（默认 4px）。" }
      : { en: "The control uses the host theme’s radius.md (8px by default).", zh: "控件使用宿主主题的 radius.md，默认 8px。" };
    const copy = (locale: Locale) => ({
      lead:entry.lead[locale],when:[entry.when[locale]],not:[locale==="zh"?"单一同步命令可直接使用基础按钮。":"Use Button for a simple synchronous command."],
      apiLead:entry.rule[locale], motion:[entry.motion[locale], shape[locale]], ai:[`kui add ${entry.id}`],
      props:[{name:entry.method,type:entry.signature,description:entry.rule[locale]},{name:"event",type:entry.event,description:locale==="en"?"Subscribe on the component entity.":"订阅控件实体的事件。"}]
    });
    return {id:entry.id as ButtonDocId, title:entry.title, rustType:entry.rustType,installable:true,story:true,usageRust:entry.usageRust,installShell:`kui add ${entry.id}`,keyboard:entry.rule,copy:{zh:copy("zh"),en:copy("en")}};
  }),
];

type CollectionEntry = (typeof collections)[number] & { avoid?: Record<Locale, string> };
const collectionEntries: CollectionEntry[] = [...collections, ...fields, ...workspace];

export const COMPONENT_CATALOG: ButtonDoc[] = [...BUTTONS_CATALOG, ...collectionEntries.map(entry => {
  const copy = (locale: Locale) => ({lead: entry.lead[locale], when: [entry.lead[locale]], not: [entry.avoid?.[locale] ?? (locale === 'zh' ? '直接执行的单一命令使用按钮。' : 'Use a button for a single immediate command.')],
    apiLead: entry.api[locale], motion: [entry.motion[locale]], ai: [`kui add ${entry.id}`],
    props: [{name:entry.method, type:entry.signature, description:entry.api[locale]}, {name:'event',type:entry.event,description:locale === 'zh' ? '订阅控件实体事件。' : 'Subscribe on the control entity.'}]});
  return {id: entry.id as ButtonDocId,group:entry.group as ButtonDoc['group'],title:entry.title,installable:true,story:true,
    rustType:entry.rustType,usageRust:entry.usageRust,installShell:`kui add ${entry.id}`,keyboard:entry.rule,copy:{en:copy('en'),zh:copy('zh')}};
})];
export function getButtonDoc(id: string): ButtonDoc | undefined {
  return COMPONENT_CATALOG.find((doc) => doc.id === id);
}

export function buttonsNav(locale: Locale): { id: ButtonDocId; title: string }[] {
  return BUTTONS_CATALOG.map(({ id, title }) => ({ id, title: title[locale] }));
}

export function buttonTitle(doc: ButtonDoc, locale: Locale): string {
  return doc.title[locale];
}
