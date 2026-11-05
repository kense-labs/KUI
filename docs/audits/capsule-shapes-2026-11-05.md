# 胶囊开关形态恢复

日期：2026-11-05。用户要求：截图所示的胶囊按钮开关恢复原本形态，不再强制8px，并把规则写进Markdown。

## 规则与覆盖

权威规则已写入根目录 `DESIGN.md` 的「胶囊开关例外 / Capsule controls」，`AGENTS.md` 同步提示。普通矩形控件仍使用8px；本记录不修改历史验收结论或把全部控件改成胶囊。

| 范围 | 恢复内容 |
| --- | --- |
| `Switch` | 42×24px轨道默认`radius.full`，两端为12px半圆；18px圆形滑块及按压伸长保持既有几何。 |
| 内联编辑 | 移除失败模拟轨道的8px圆角，恢复胶囊。 |
| ActionSwap演示 | 移除模拟网络中断Switch的`with_track_radius(radius.md)`覆盖，使用共享Switch默认胶囊。 |
| 设置和Cookie同意 | 使用共享Switch，自动获得同一胶囊轨道；禁用和事件逻辑保持原实现。 |
| `BillingToggle` | 外轨道、选项、移动高亮与优惠标签均为胶囊；高亮边框与阴影沿当前实际轮廓绘制。 |
| `GlassSegmented` | 玻璃轨道、选项与伸缩高亮均读取`radius.full`。 |

基础Switch、计费周期、玻璃分段的组件Markdown和中英文网页文档已同步。保留`Switch::with_track_radius`的显式宿主接口，但产品演示不再强制8px。

## 公开依据

- [Switch公开注册表](https://uiarc.dev/r/switch.json)：轨道和滑块均为`radius-pill`；轨道42×24、padding3、滑块18×18。JSON SHA-256：`bc3b514941718cf6d057a1fb179ffd21eefeb71e7547043177879fe7828fd7c8`。
- [BillingToggle公开注册表](https://uiarc.dev/r/billing-toggle.json)：root、thumb、option、badge均为`radius-pill`。JSON SHA-256：`3ff007b6378dafb7525f6ea7b5ce9e70271225c3b614d957cf25787e9037b901`。
- [GlassSegmented公开样式](https://uiarc.dev/_next/static/css/0dc30b4f3982f43e.css)：root、segment、badge均为`radius-pill`；官网实时页面测得root高44px、segment高34px，计算圆角均9999px。仅据公开样式和实物核对，不涉及付费源码下载。
- SegmentedControl、ToggleGroup公开样式使用`radius-control`而非`radius-pill`；TldrToggle的模式切换复用SegmentedControl，因此这三项继续遵循KUI矩形8px默认值。

## 验证记录

本轮没有提交、推送或线上发布。本次已完成以下检查；未重新宣称全部组件视觉或60fps验收通过。


- 原生组件185项全部通过（包含玻璃分段拖动、触摸取消和计费高亮几何）；Switch定向story的3项通过。
- Web库无默认特性检查、Web宿主检查和优化WASM构建通过。实际预览修订为`271d9fd5d85d8e9ff0e0`。
- 在空白项目分别经CLI安装`switch`、`billing-toggle`、`glass-segmented`，doctor、原生与WASM离线编译均通过；注册表中的三份组件源码与当前registry源文件逐字一致。
- `kui-build verify-preview`通过，234个文件纳入哈希；网站输入校验以及193项双语文档、795个静态目标检查通过。
- 实际浏览器确认：内联编辑开关关态和开态的两端为半圆；计费按月/按年切换后高亮保持胶囊，亮暗主题边缘正常；玻璃分段选到末项后高亮仍为胶囊；320px中文页面无横向溢出，控件完整显示，页面和预览均可滚动。临时视口设置已恢复，主题回到原暗色。
- 仅浏览器画面的实拍保存在`/private/tmp/kui-capsule-inline-dark-final.jpg`、`/private/tmp/kui-capsule-inline-detail.jpg`、`/private/tmp/kui-capsule-billing-yearly.jpg`、`/private/tmp/kui-capsule-billing-light.jpg`、`/private/tmp/kui-capsule-glass-light.jpg`、`/private/tmp/kui-capsule-glass-320.jpg`。

### 全仓库检查的未通过项

- Story整套106项中105项通过，`extra_story::inputs::glass_menu_tests::presence_overflow_roster_keyboard_selection_and_host_updates`在`inputs.rs:2014`未找到`Alex Chen (you) · Active now`调试节点。本次未修改PresenceStack或该测试，不把整套story记为全绿。
- Clippy在本次未修改的Carousel、CookieConsent、SheetStack、Ridgeline和AgentLoop中报告共6个lint错误（manual_clamp、if_same_then_else、collapsible_if、unnecessary_cast）。原始记录：`/private/tmp/kui-capsule-clippy.log`。
- 全仓库fmt报告其他文件的格式差异；本次涉及的5个Rust文件单独rustfmt检查通过。未对其他Agent的文件执行自动格式化。
