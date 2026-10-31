/** Buttons family catalog — docs metadata for the website. Only `button` is installable today. */

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
  | "theme-switch";

export type Locale = "zh" | "en";

export type PropRow = { name: string; type: string; default?: string; description: string };

export type ButtonDoc = {
  id: ButtonDocId;
  /** Localized nav / page title. */
  title: Record<Locale, string>;
  installable: boolean;
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

const futureInstall = (id: string) =>
  `# Not in the MVP registry yet; documents the Buttons family contract.
kui add ${id}`;

export const BUTTONS_CATALOG: ButtonDoc[] = [
  {
    id: "button",
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
        lead: "Finished KUI button with clear control rhythm (44px md, 8px radius, medium weight, outlined danger) on pure black/white tokens.",
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
          row("loading", "bool", "Shows an ellipsis and blocks clicks.", "false"),
          row("disabled", "bool", "Blocks clicks.", "false"),
          row("on_click", "Fn(&ClickEvent, &mut Window, &mut App)", "Click handler."),
          row("into_themed(cx)", "&App → Stateful<Div>", "Applies KUI theme tokens."),
        ],
      },
      zh: {
        lead: "KUI 成品按钮：节奏清晰（md 44px、8px 圆角、中等字重、描边 danger），基于纯黑白令牌。",
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
          row("loading", "bool", "显示省略号并阻止点击。", "false"),
          row("disabled", "bool", "阻止点击。", "false"),
          row("on_click", "Fn(&ClickEvent, &mut Window, &mut App)", "点击处理函数。"),
          row("into_themed(cx)", "&App → Stateful<Div>", "应用 KUI 主题令牌。"),
        ],
      },
    },
  },
  {
    id: "action-button",
    title: { en: "Action button", zh: "操作按钮" },
    installable: false,
    rustType: "ActionButton",
    usageRust: `use kui::components::action_button::{ActionButton, CommitError};

ActionButton::new("publish")
    .label("Publish")
    .pending_label("Publishing")
    .success_label("Published")
    .on_commit(|_window, cx| {
        cx.background_spawn(async move { publish().await.map_err(CommitError::from) })
    })
    .into_themed(cx);`,
    installShell: futureInstall("action-button"),
    copy: {
      en: {
        lead: "Button that runs an async commit and shows pending, success, or failure on the control itself.",
        when: [
          "A click starts work that takes time: save, publish, sync, send.",
          "The result belongs on the button, not in a separate toast.",
        ],
        not: [
          "Instant, synchronous actions; use Button.",
          "Work that needs a progress bar or a log.",
        ],
        apiLead: "Returns a Task from on_commit. Phase is owned by the control unless you pass ActionPhase.",
        motion: [
          "Label swaps use MOTION.morph with width held steady.",
          "Press feedback uses MOTION.snappy.",
          "Swaps instantly when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add action-button` over wrapping Button with your own state.",
          "Never block inside on_commit; return a Task.",
          "Use ActionPhase instead of bool pairs.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id; owns phase state."),
          row("label", "impl Into<SharedString>", "Idle text.", '""'),
          row("pending_label / success_label / failed_label", "impl Into<SharedString>", "In-place text per phase.", "idle label"),
          row("on_commit", "Fn(&mut Window, &mut App) -> Task<Result<(), CommitError>>", "Starts the async work."),
          row("settle_after", "Duration", "Time in success or failed before idle.", "1500ms"),
          row("phase", "ActionPhase", "Controlled phase: Idle, Pending, Success, Failed."),
          row("disabled", "bool", "Blocks commits.", "false"),
        ],
      },
      zh: {
        lead: "执行异步提交的按钮，在控件自身上显示进行中、成功或失败。",
        when: ["点击后启动耗时工作：保存、发布、同步、发送。", "结果应显示在按钮上，而不是另起一个提示。"],
        not: ["即时的同步操作，请使用 Button。", "需要进度条或日志的工作。"],
        apiLead: "on_commit 返回 Task。除非传入 ActionPhase，否则阶段由控件自身管理。",
        motion: [
          "文字切换使用 MOTION.morph，并保持宽度稳定。",
          "按压反馈使用 MOTION.snappy。",
          "Theme.reduced_motion 为 true 时即时切换。",
        ],
        ai: [
          "优先使用 `kui add action-button`，不要用自己的状态包装 Button。",
          "不要在 on_commit 内阻塞；返回 Task。",
          "使用 ActionPhase，而不是布尔值组合。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id；持有阶段状态。"),
          row("label", "impl Into<SharedString>", "空闲时的文字。", '""'),
          row("pending_label / success_label / failed_label", "impl Into<SharedString>", "各阶段的原位文字。", "空闲文字"),
          row("on_commit", "Fn(&mut Window, &mut App) -> Task<Result<(), CommitError>>", "启动异步工作。"),
          row("settle_after", "Duration", "成功或失败状态保持多久后回到空闲。", "1500ms"),
          row("phase", "ActionPhase", "受控阶段：Idle、Pending、Success、Failed。"),
          row("disabled", "bool", "阻止提交。", "false"),
        ],
      },
    },
  },
  {
    id: "split-button",
    title: { en: "Split button", zh: "分裂按钮" },
    installable: false,
    rustType: "SplitButton",
    usageRust: `use kui::components::split_button::{SplitButton, SplitItem};

SplitButton::new("merge")
    .label("Merge")
    .item(SplitItem::new("squash", "Squash and merge"))
    .item(SplitItem::new("rebase", "Rebase and merge"))
    .on_primary(|_window, _cx| merge_default())
    .on_select(|item, _window, _cx| merge_with(item))
    .into_themed(cx);`,
    installShell: futureInstall("split-button"),
    copy: {
      en: {
        lead: "Primary action joined to a menu of nearby alternatives.",
        when: [
          "One action is the common case and two to five related variants stay one click away.",
          "Merge strategies, export formats, send options.",
        ],
        not: [
          "Unrelated alternatives; use Button group or a menu.",
          "A single action; use Button.",
        ],
        apiLead: "Two focus stops: the primary segment and the chevron. The menu holds alternatives only.",
        motion: [
          "Menu open uses MOTION.snappy (short fade and 4px rise).",
          "The 1px divider between segments never animates.",
          "Appears without movement when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add split-button` over two Buttons placed side by side.",
          "Keep the primary action reachable by its own label.",
          "Name the chevron from the label, for example \"More merge options\".",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id; the menu id derives from it."),
          row("label", "impl Into<SharedString>", "Primary segment text.", '""'),
          row("item", "SplitItem", "Menu entry; SplitItem::new(id, label)."),
          row("primary / secondary / ghost", "ButtonVariant", "Variant; danger is not offered.", "primary"),
          row("on_primary", "Fn(&mut Window, &mut App)", "Fires from the main segment."),
          row("on_select", "Fn(&SplitItem, &mut Window, &mut App)", "Receives the chosen item."),
          row("open", "bool", "Controlled menu state.", "uncontrolled"),
        ],
      },
      zh: {
        lead: "主操作与一组相近备选操作相连的分裂按钮。",
        when: ["有一个常用操作，另有两到五个相关变体需要一键可达。", "合并方式、导出格式、发送选项。"],
        not: ["彼此无关的备选项，请使用 Button group 或菜单。", "只有单一操作，请使用 Button。"],
        apiLead: "有两个焦点停靠点：主分段与箭头。菜单只放备选操作。",
        motion: [
          "菜单展开使用 MOTION.snappy（短淡入并上浮 4px）。",
          "分段之间 1px 的分隔线不做动画。",
          "Theme.reduced_motion 为 true 时直接出现，无位移。",
        ],
        ai: [
          "优先使用 `kui add split-button`，不要并排放两个 Button。",
          "主操作仍需能通过自身标签触达。",
          "箭头的名称由标签派生，例如“更多合并选项”。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id；菜单 id 由其派生。"),
          row("label", "impl Into<SharedString>", "主分段文字。", '""'),
          row("item", "SplitItem", "菜单项；SplitItem::new(id, label)。"),
          row("primary / secondary / ghost", "ButtonVariant", "变体；不提供 danger。", "primary"),
          row("on_primary", "Fn(&mut Window, &mut App)", "由主分段触发。"),
          row("on_select", "Fn(&SplitItem, &mut Window, &mut App)", "接收所选菜单项。"),
          row("open", "bool", "受控的菜单状态。", "非受控"),
        ],
      },
    },
  },
  {
    id: "button-group",
    title: { en: "Button group", zh: "按钮组" },
    installable: false,
    rustType: "ButtonGroup",
    usageRust: `use kui::components::button_group::{ButtonGroup, GroupItem};

ButtonGroup::new("view")
    .item(GroupItem::new("list", "List"))
    .item(GroupItem::new("grid", "Grid"))
    .selected("list")
    .on_change(|id, _window, _cx| set_view(id))
    .into_themed(cx);`,
    installShell: futureInstall("button-group"),
    copy: {
      en: {
        lead: "Related actions on one surface with a shared highlight.",
        when: [
          "A small set of peer actions or modes that read as one control.",
          "View toggles, alignment, range pickers.",
        ],
        not: [
          "Unrelated actions; use separate Buttons.",
          "Page navigation.",
          "Items that should stay visually separate; use Floating button group.",
        ],
        apiLead: "Items share one border and one highlight. Tab enters the group once; arrows move between items.",
        motion: [
          "The shared highlight slides between items with MOTION.morph.",
          "Outer corners keep the 8px radius; inner edges are square with 1px dividers.",
          "The highlight jumps when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add button-group` over rows of buttons with hand-rolled borders.",
          "Keep item ids stable; the highlight tracks them.",
          "Use pure black/white tokens for the highlight.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id."),
          row("item", "GroupItem", "GroupItem::new(id, label)."),
          row("selected", "impl Into<ElementId>", "Controlled selected item id."),
          row("selection", "GroupSelection · Single | None", "Whether items hold a selection.", "Single"),
          row("with_size", "Size · Sm | Md | Lg", "Control height and padding.", "Md"),
          row("disabled", "bool", "Blocks the whole group.", "false"),
          row("on_change", "Fn(&ElementId, &mut Window, &mut App)", "Receives the activated item id."),
        ],
      },
      zh: {
        lead: "同一表面上的相关操作，共享一个高亮。",
        when: ["一小组并列的操作或模式，视觉上是同一个控件。", "视图切换、对齐方式、范围选择。"],
        not: ["彼此无关的操作，请使用独立的 Button。", "页面导航。", "需要保持视觉分离的项目，请使用 Floating button group。"],
        apiLead: "各项共用一条边框和一个高亮。Tab 只进入分组一次，方向键在项之间移动。",
        motion: [
          "共享高亮使用 MOTION.morph 在项之间滑动。",
          "外侧保持 8px 圆角；内侧为直角，以 1px 分隔线隔开。",
          "Theme.reduced_motion 为 true 时高亮直接跳转。",
        ],
        ai: [
          "优先使用 `kui add button-group`，不要用手写边框拼按钮行。",
          "保持各项 id 稳定；高亮依赖它们追踪。",
          "高亮使用纯黑白令牌。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id。"),
          row("item", "GroupItem", "GroupItem::new(id, label)。"),
          row("selected", "impl Into<ElementId>", "受控的选中项 id。"),
          row("selection", "GroupSelection · Single | None", "各项是否保持选中状态。", "Single"),
          row("with_size", "Size · Sm | Md | Lg", "控件高度与内边距。", "Md"),
          row("disabled", "bool", "阻止整个分组。", "false"),
          row("on_change", "Fn(&ElementId, &mut Window, &mut App)", "接收被激活项的 id。"),
        ],
      },
    },
  },
  {
    id: "floating-button-group",
    title: { en: "Floating button group", zh: "浮动按钮组" },
    installable: false,
    rustType: "FloatingButtonGroup",
    usageRust: `use kui::components::floating_button_group::{FloatItem, FloatingButtonGroup};

FloatingButtonGroup::new("tools")
    .item(FloatItem::new("select", "Select"))
    .item(FloatItem::new("draw", "Draw"))
    .item(FloatItem::new("erase", "Erase"))
    .selected("select")
    .on_change(|id, _window, _cx| set_tool(id))
    .into_themed(cx);`,
    installShell: futureInstall("floating-button-group"),
    copy: {
      en: {
        lead: "Soft, separate buttons in a quiet tray with a morphing highlight.",
        when: [
          "Secondary toolbars over content: editor tools, canvas controls, media actions.",
          "Buttons should stay distinct while the tray stays calm.",
        ],
        not: [
          "Form actions or primary calls to action.",
          "Items that should read as one joined control; use Button group.",
        ],
        apiLead: "A toolbar with a 1px hairline tray. Items stay separate; one highlight morphs between them.",
        motion: [
          "The highlight morphs with MOTION.morph, changing position and width together.",
          "Hover lifts with MOTION.snappy.",
          "The highlight swaps in place when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add floating-button-group` over absolute-positioned Button rows.",
          "Use black/white tokens; do not tint the tray with the Kense brand color.",
          "Icon-only items must set an accessible label.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id."),
          row("item", "FloatItem", "FloatItem::new(id, label); optional .icon(IconName)."),
          row("selected", "impl Into<ElementId>", "Controlled item id."),
          row("with_size", "Size · Sm | Md | Lg", "Control height and padding.", "Md"),
          row("gap", "Pixels", "Space between buttons.", "4px"),
          row("disabled", "bool", "Blocks the whole tray.", "false"),
          row("on_change", "Fn(&ElementId, &mut Window, &mut App)", "Receives the activated item id."),
        ],
      },
      zh: {
        lead: "安静托盘中相互独立的柔和按钮，带有形变高亮。",
        when: ["内容之上的次要工具栏：编辑器工具、画布控件、媒体操作。", "按钮保持各自独立，托盘本身保持安静。"],
        not: ["表单操作或主要行动号召。", "需要看作一个整体的控件，请使用 Button group。"],
        apiLead: "带 1px 细线边框托盘的工具栏。各项保持独立，一个高亮在它们之间形变。",
        motion: [
          "高亮使用 MOTION.morph 形变，位置与宽度同时变化。",
          "悬停上浮使用 MOTION.snappy。",
          "Theme.reduced_motion 为 true 时高亮原地切换。",
        ],
        ai: [
          "优先使用 `kui add floating-button-group`，不要用绝对定位的 Button 行。",
          "使用黑白令牌；不要用 Kense 品牌色给托盘着色。",
          "仅图标的项必须设置可访问标签。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id。"),
          row("item", "FloatItem", "FloatItem::new(id, label)；可选 .icon(IconName)。"),
          row("selected", "impl Into<ElementId>", "受控的项 id。"),
          row("with_size", "Size · Sm | Md | Lg", "控件高度与内边距。", "Md"),
          row("gap", "Pixels", "按钮之间的间距。", "4px"),
          row("disabled", "bool", "阻止整个托盘。", "false"),
          row("on_change", "Fn(&ElementId, &mut Window, &mut App)", "接收被激活项的 id。"),
        ],
      },
    },
  },
  {
    id: "expanding-button-group",
    title: { en: "Expanding button group", zh: "展开按钮组" },
    installable: false,
    rustType: "ExpandingButtonGroup",
    usageRust: `use kui::components::expanding_button_group::{ExpandItem, ExpandingButtonGroup};

ExpandingButtonGroup::new("row-actions")
    .item(ExpandItem::new("edit", IconName::Pencil, "Edit"))
    .item(ExpandItem::new("share", IconName::Share, "Share"))
    .item(ExpandItem::new("delete", IconName::Trash, "Delete"))
    .on_activate(|id, _window, _cx| run_action(id))
    .into_themed(cx);`,
    installShell: futureInstall("expanding-button-group"),
    copy: {
      en: {
        lead: "Compact icon bar that reveals labels on hover or focus.",
        when: [
          "Space is tight and the actions are familiar: row actions, card toolbars, dense editors.",
        ],
        not: [
          "Icons are not self-evident and labels must always be visible; use Button group.",
          "Touch-only layouts where hover never occurs.",
        ],
        apiLead: "Every item has an icon and a label. Focus expands the same label that hover does.",
        motion: [
          "Labels expand with MOTION.morph (width and opacity together); neighbors shift without overlap.",
          "Press uses MOTION.snappy.",
          "Labels appear without width animation when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add expanding-button-group` over hand-rolled hover reveals.",
          "Always provide a label per item.",
          "Reserve the expanded width so layout does not jump.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id."),
          row("item", "ExpandItem", "ExpandItem::new(id, icon, label)."),
          row("expand_on", "ExpandTrigger · HoverOrFocus | Focus | Always", "When labels expand.", "HoverOrFocus"),
          row("with_size", "Size · Sm | Md | Lg", "Control height and padding.", "Md"),
          row("disabled", "bool", "Blocks the whole bar.", "false"),
          row("on_activate", "Fn(&ElementId, &mut Window, &mut App)", "Receives the activated item id."),
        ],
      },
      zh: {
        lead: "紧凑的图标栏，悬停或聚焦时展开标签。",
        when: ["空间紧张且操作为人熟知：行操作、卡片工具栏、密集编辑器。"],
        not: ["图标含义不明显、标签必须始终可见时，请使用 Button group。", "没有悬停的纯触控布局。"],
        apiLead: "每一项都有图标和标签。聚焦展开的标签与悬停时相同。",
        motion: [
          "标签使用 MOTION.morph 展开（宽度与不透明度同时变化）；相邻项平移而不重叠。",
          "按压使用 MOTION.snappy。",
          "Theme.reduced_motion 为 true 时标签直接出现，无宽度动画。",
        ],
        ai: [
          "优先使用 `kui add expanding-button-group`，不要手写悬停展开。",
          "每一项都必须提供标签。",
          "预留展开后的宽度，避免布局跳动。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id。"),
          row("item", "ExpandItem", "ExpandItem::new(id, icon, label)。"),
          row("expand_on", "ExpandTrigger · HoverOrFocus | Focus | Always", "标签何时展开。", "HoverOrFocus"),
          row("with_size", "Size · Sm | Md | Lg", "控件高度与内边距。", "Md"),
          row("disabled", "bool", "阻止整个栏。", "false"),
          row("on_activate", "Fn(&ElementId, &mut Window, &mut App)", "接收被激活项的 id。"),
        ],
      },
    },
  },
  {
    id: "copy-button",
    title: { en: "Copy button", zh: "复制按钮" },
    installable: false,
    rustType: "CopyButton",
    usageRust: `use kui::components::copy_button::CopyButton;

CopyButton::new("copy-install")
    .text("kui add button")
    .label("Copy")
    .on_result(|result, _window, _cx| log_copy(result))
    .into_themed(cx);`,
    installShell: futureInstall("copy-button"),
    copy: {
      en: {
        lead: "Clipboard copy with Copied or Failed shown in place.",
        when: [
          "Next to values users paste elsewhere: tokens, install commands, ids, snippets.",
        ],
        not: [
          "Actions that are not clipboard writes; use Button or Action button.",
          "Copying secrets the user did not ask for.",
        ],
        apiLead: "Writes text to the clipboard and reports Result<(), CopyError>. Web writes can fail and show Failed.",
        motion: [
          "Icon and label swap with MOTION.morph.",
          "Press uses MOTION.snappy.",
          "The result replaces idle instantly when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add copy-button` over custom clipboard wiring.",
          "Always handle CopyError; do not assume success.",
          "Keep the control width stable between states.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id."),
          row("text", "impl Into<SharedString>", "Value written to the clipboard."),
          row("label", "impl Into<SharedString>", "Idle text.", '"Copy"'),
          row("copied_label / failed_label", "impl Into<SharedString>", "In-place result text.", '"Copied" / "Failed"'),
          row("icon_only", "bool", "Shows only the icon; label stays the accessible name.", "false"),
          row("reset_after", "Duration", "Time before returning to idle.", "1500ms"),
          row("on_result", "Fn(Result<(), CopyError>, &mut Window, &mut App)", "Receives the copy outcome."),
        ],
      },
      zh: {
        lead: "复制到剪贴板，并在原位显示“已复制”或“失败”。",
        when: ["放在用户要粘贴到别处的值旁：令牌、安装命令、id、代码片段。"],
        not: ["不是写入剪贴板的操作，请使用 Button 或 Action button。", "用户并未要求时复制机密信息。"],
        apiLead: "把文本写入剪贴板并返回 Result<(), CopyError>。Web 端写入可能失败并显示 Failed。",
        motion: [
          "图标与文字使用 MOTION.morph 切换。",
          "按压使用 MOTION.snappy。",
          "Theme.reduced_motion 为 true 时结果状态即时替换空闲状态。",
        ],
        ai: [
          "优先使用 `kui add copy-button`，不要自写剪贴板逻辑。",
          "始终处理 CopyError，不要假定成功。",
          "各状态之间保持控件宽度稳定。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id。"),
          row("text", "impl Into<SharedString>", "写入剪贴板的值。"),
          row("label", "impl Into<SharedString>", "空闲时的文字。", '"Copy"'),
          row("copied_label / failed_label", "impl Into<SharedString>", "原位结果文字。", '"Copied" / "Failed"'),
          row("icon_only", "bool", "只显示图标；标签仍作为可访问名称。", "false"),
          row("reset_after", "Duration", "多久后回到空闲。", "1500ms"),
          row("on_result", "Fn(Result<(), CopyError>, &mut Window, &mut App)", "接收复制结果。"),
        ],
      },
    },
  },
  {
    id: "action-swap",
    title: { en: "Action swap", zh: "操作切换" },
    installable: false,
    rustType: "ActionSwap",
    usageRust: `use kui::components::action_swap::{ActionSwap, SwapState};

ActionSwap::new("follow")
    .state(SwapState::new("follow", "Follow").primary())
    .state(SwapState::new("following", "Following").secondary())
    .hold_for(Duration::from_millis(800))
    .current("follow")
    .on_swap(|next, _window, _cx| set_follow(next))
    .into_themed(cx);`,
    installShell: futureInstall("action-swap"),
    copy: {
      en: {
        lead: "Button that cycles through related states, such as Follow and Following.",
        when: [
          "A reversible two- or three-state action where the label reflects current state.",
          "Follow, subscribe, save, mute.",
        ],
        not: [
          "Destructive actions; use Confirm morph.",
          "Async commits; use Action button.",
          "A pure on/off icon; use Icon morph.",
        ],
        apiLead: "Each state has an id, label, and variant. hold_for delays a swap so it can be cancelled.",
        motion: [
          "Labels swap with MOTION.morph and the width interpolates between states.",
          "Variant fill changes with MOTION.snappy.",
          "Label and fill change instantly when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add action-swap` over flipping a Button label by hand.",
          "Use state ids, not magic strings in labels.",
          "Keep two to three states.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id."),
          row("state", "SwapState", "SwapState::new(id, label); variant per state."),
          row("current", "impl Into<ElementId>", "Controlled state id.", "first state"),
          row("hold_for", "Duration", "Delay before a swap commits, so a press can be undone."),
          row("with_size", "Size · Sm | Md | Lg", "Control height and padding.", "Md"),
          row("disabled", "bool", "Blocks swaps.", "false"),
          row("on_swap", "Fn(&ElementId, &mut Window, &mut App)", "Receives the next state id."),
        ],
      },
      zh: {
        lead: "在相关状态之间循环的按钮，例如 Follow 与 Following。",
        when: ["可逆的两到三态操作，标签反映当前状态。", "关注、订阅、收藏、静音。"],
        not: ["破坏性操作，请使用 Confirm morph。", "异步提交，请使用 Action button。", "纯开关图标，请使用 Icon morph。"],
        apiLead: "每个状态有 id、标签和变体。hold_for 会延迟切换，使其可被取消。",
        motion: [
          "文字使用 MOTION.morph 切换，宽度在状态之间插值。",
          "变体填充使用 MOTION.snappy 变化。",
          "Theme.reduced_motion 为 true 时文字与填充即时变化。",
        ],
        ai: [
          "优先使用 `kui add action-swap`，不要手动翻转 Button 的标签。",
          "使用状态 id，不要把标签当作魔法字符串。",
          "保持两到三个状态。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id。"),
          row("state", "SwapState", "SwapState::new(id, label)；每个状态有各自变体。"),
          row("current", "impl Into<ElementId>", "受控的状态 id。", "第一个状态"),
          row("hold_for", "Duration", "切换生效前的延迟，使一次按压可被撤销。"),
          row("with_size", "Size · Sm | Md | Lg", "控件高度与内边距。", "Md"),
          row("disabled", "bool", "阻止切换。", "false"),
          row("on_swap", "Fn(&ElementId, &mut Window, &mut App)", "接收下一个状态的 id。"),
        ],
      },
    },
  },
  {
    id: "icon-morph",
    title: { en: "Icon morph", zh: "图标变形" },
    installable: false,
    rustType: "IconMorph",
    usageRust: `use kui::components::icon_morph::IconMorph;

IconMorph::new("bookmark")
    .off(IconName::Bookmark)
    .on(IconName::BookmarkFilled)
    .label("Bookmark")
    .active(is_saved)
    .on_toggle(|next, _window, _cx| set_saved(next))
    .into_themed(cx);`,
    installShell: futureInstall("icon-morph"),
    copy: {
      en: {
        lead: "Two-state icon button whose glyph morphs between on and off.",
        when: [
          "Compact toggles with a well-known glyph pair: bookmark, mute, play and pause, menu and close.",
        ],
        not: [
          "The state needs a text label; use Action swap.",
          "One-way actions; use Button.",
        ],
        apiLead: "Two icons, one accessible label. The label does not change with state; pressed state is exposed.",
        motion: [
          "Glyphs cross-morph with MOTION.morph: stroke fade and scale shift.",
          "Press uses MOTION.snappy.",
          "The icon swaps with no morph when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add icon-morph` over swapping two Button icons by hand.",
          "Pick icon pairs that share a bounding box.",
          "Never ship an icon-only control without a label.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id."),
          row("off / on", "IconName", "Icon for each state."),
          row("label", "impl Into<SharedString>", "Accessible name; also the tooltip."),
          row("active", "bool", "Controlled on state.", "false"),
          row("ghost / secondary", "ButtonVariant", "Variant.", "ghost"),
          row("with_size", "Size · Sm | Md | Lg", "Square hit target.", "Md"),
          row("on_toggle", "Fn(bool, &mut Window, &mut App)", "Receives the next state."),
        ],
      },
      zh: {
        lead: "双态图标按钮，图标在开与关之间形变。",
        when: ["使用常见图标对的紧凑开关：收藏、静音、播放与暂停、菜单与关闭。"],
        not: ["状态需要文字标签时，请使用 Action swap。", "单向操作，请使用 Button。"],
        apiLead: "两个图标，一个可访问标签。标签不随状态变化；会暴露按下状态。",
        motion: [
          "图标使用 MOTION.morph 交叉形变：笔画淡入淡出并缩放。",
          "按压使用 MOTION.snappy。",
          "Theme.reduced_motion 为 true 时图标直接切换，无形变。",
        ],
        ai: [
          "优先使用 `kui add icon-morph`，不要手动切换两个 Button 图标。",
          "选择外框相同的图标对。",
          "不要发布没有标签的纯图标控件。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id。"),
          row("off / on", "IconName", "各状态对应的图标。"),
          row("label", "impl Into<SharedString>", "可访问名称，同时作为提示。"),
          row("active", "bool", "受控的开启状态。", "false"),
          row("ghost / secondary", "ButtonVariant", "变体。", "ghost"),
          row("with_size", "Size · Sm | Md | Lg", "方形点击区域。", "Md"),
          row("on_toggle", "Fn(bool, &mut Window, &mut App)", "接收下一个状态。"),
        ],
      },
    },
  },
  {
    id: "confirm-morph",
    title: { en: "Confirm morph", zh: "确认变形" },
    installable: false,
    rustType: "ConfirmMorph",
    usageRust: `use kui::components::confirm_morph::ConfirmMorph;

ConfirmMorph::new("delete")
    .label("Delete")
    .confirm_label("Confirm")
    .undo_label("Undo")
    .undo_for(Duration::from_secs(5))
    .on_confirm(|_window, _cx| schedule_delete())
    .on_undo(|_window, _cx| cancel_delete())
    .into_themed(cx);`,
    installShell: futureInstall("confirm-morph"),
    copy: {
      en: {
        lead: "Destructive button that morphs into a confirm, then into Undo.",
        when: [
          "Deletes and resets where one extra step is enough: remove item, clear history, revoke key.",
        ],
        not: [
          "Irreversible actions that need typed confirmation; use a dialog.",
          "Non-destructive toggles; use Action swap.",
        ],
        apiLead: "Three phases: idle, armed, undo. Run the destructive work only from on_confirm.",
        motion: [
          "The button morphs between phases with MOTION.morph, holding height and animating width.",
          "A thin countdown line shows the undo window.",
          "Labels swap instantly and the line is static when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add confirm-morph` over a modal for simple deletes.",
          "Run the real work only from on_confirm.",
          "Keep the outlined danger style on pure black/white tokens.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id; owns ConfirmPhase."),
          row("label / confirm_label / undo_label", "impl Into<SharedString>", "Text for idle, armed, and undo.", '"" / "Confirm" / "Undo"'),
          row("arm_timeout", "Duration", "Armed state reverts to idle.", "3s"),
          row("undo_for", "Duration", "Undo window after confirm.", "5s"),
          row("with_size", "Size · Sm | Md | Lg", "Control height and padding.", "Md"),
          row("on_confirm", "Fn(&mut Window, &mut App)", "Fires when the undo window ends."),
          row("on_undo", "Fn(&mut Window, &mut App)", "Fires when Undo is chosen."),
        ],
      },
      zh: {
        lead: "破坏性按钮，先形变为确认，再形变为撤销。",
        when: ["多一步确认就够的删除与重置：移除条目、清空历史、吊销密钥。"],
        not: ["需要输入文字确认的不可逆操作，请使用对话框。", "非破坏性开关，请使用 Action swap。"],
        apiLead: "三个阶段：空闲、待确认、撤销。只在 on_confirm 中执行破坏性工作。",
        motion: [
          "按钮使用 MOTION.morph 在阶段之间形变，保持高度、动画宽度。",
          "一条细倒计时线表示撤销窗口。",
          "Theme.reduced_motion 为 true 时文字即时切换，倒计时线静止。",
        ],
        ai: [
          "简单删除优先使用 `kui add confirm-morph`，而不是模态框。",
          "只在 on_confirm 中执行真正的工作。",
          "在纯黑白令牌上保持描边 danger 样式。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id；持有 ConfirmPhase。"),
          row("label / confirm_label / undo_label", "impl Into<SharedString>", "空闲、待确认与撤销阶段的文字。", '"" / "Confirm" / "Undo"'),
          row("arm_timeout", "Duration", "待确认状态多久后回到空闲。", "3s"),
          row("undo_for", "Duration", "确认后的撤销窗口。", "5s"),
          row("with_size", "Size · Sm | Md | Lg", "控件高度与内边距。", "Md"),
          row("on_confirm", "Fn(&mut Window, &mut App)", "撤销窗口结束时触发。"),
          row("on_undo", "Fn(&mut Window, &mut App)", "选择撤销时触发。"),
        ],
      },
    },
  },
  {
    id: "theme-switch",
    title: { en: "Theme switcher", zh: "主题切换" },
    installable: false,
    rustType: "ThemeSwitch",
    usageRust: `use kui::components::theme_switch::ThemeSwitch;
use kui::theme::ThemeMode;

ThemeSwitch::new("theme")
    .mode(ThemeMode::Dark)
    .on_change(|mode, _window, cx| apply_theme(mode, cx))
    .into_themed(cx);`,
    installShell: futureInstall("theme-switch"),
    copy: {
      en: {
        lead: "Light and dark toggle for the host theme.",
        when: [
          "Users choose light or dark mode in app chrome: headers, settings, docs sites.",
        ],
        not: [
          "Custom palettes or brand themes; KUI chrome is black and white.",
          "A general two-state toggle; use Icon morph.",
        ],
        apiLead: "Switches between ThemeMode values through foundation. Depends on kui-foundation.",
        motion: [
          "Sun and moon glyphs morph with MOTION.morph; colors fade over a short interval.",
          "Press uses MOTION.snappy.",
          "Glyph and colors change at once when Theme.reduced_motion is true.",
        ],
        ai: [
          "Prefer `kui add theme-switch` over writing theme state yourself.",
          "Use ThemeMode, not strings or bools.",
          "Do not add a third mode here.",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "Stable id."),
          row("mode", "ThemeMode", "Controlled mode.", "current Theme mode"),
          row("on_change", "Fn(ThemeMode, &mut Window, &mut App)", "Receives the next mode.", "foundation setter"),
          row("label", "impl Into<SharedString>", "Accessible name.", '"Toggle theme"'),
          row("with_size", "Size · Sm | Md | Lg", "Square hit target.", "Md"),
          row("disabled", "bool", "Blocks toggles.", "false"),
        ],
      },
      zh: {
        lead: "宿主主题的浅色与深色切换。",
        when: ["用户在应用外壳中选择浅色或深色模式：页头、设置、文档站点。"],
        not: ["自定义调色板或品牌主题；KUI 外观为黑白。", "通用的双态开关，请使用 Icon morph。"],
        apiLead: "通过 foundation 在 ThemeMode 之间切换。依赖 kui-foundation。",
        motion: [
          "日月图标使用 MOTION.morph 形变；颜色在短时间内渐变。",
          "按压使用 MOTION.snappy。",
          "Theme.reduced_motion 为 true 时图标与颜色同时切换。",
        ],
        ai: [
          "优先使用 `kui add theme-switch`，不要自行编写主题状态。",
          "使用 ThemeMode，不要用字符串或布尔值。",
          "不要在此添加第三种模式。",
        ],
        props: [
          row("new(id)", "impl Into<ElementId>", "稳定 id。"),
          row("mode", "ThemeMode", "受控的模式。", "当前 Theme 模式"),
          row("on_change", "Fn(ThemeMode, &mut Window, &mut App)", "接收下一个模式。", "foundation 设置器"),
          row("label", "impl Into<SharedString>", "可访问名称。", '"Toggle theme"'),
          row("with_size", "Size · Sm | Md | Lg", "方形点击区域。", "Md"),
          row("disabled", "bool", "阻止切换。", "false"),
        ],
      },
    },
  },
];

export function getButtonDoc(id: string): ButtonDoc | undefined {
  return BUTTONS_CATALOG.find((doc) => doc.id === id);
}

export function buttonsNav(locale: Locale): { id: ButtonDocId; title: string }[] {
  return BUTTONS_CATALOG.map(({ id, title }) => ({ id, title: title[locale] }));
}

export function buttonTitle(doc: ButtonDoc, locale: Locale): string {
  return doc.title[locale];
}
