# KUI 新增 85 项组件重新审查清单

最后更新：2026-11-05 19:12（北京时间）。本轮状态：**0 / 85 项完成对齐验收，85 项待逐项重审**（activity-rings、sankey-flow 已完成 S/A/V 三阶段 `SAV□□□□□`；rating、radio-cards 已完成 S/A/V/I/D 五阶段 `SAVI□□D□`，M 与 P 仍待验）。这表示尚未通过本轮验收，并不表示没有实现。

本轮第二批核对（19:12）已把两项的六条「尚待核实」逐条落地：①rating 变词入场/退出模糊参数与官方 `motionTokens.blur` 逐项吻合并实测到中间帧；②说明面板退出内容交叉淡出实测到中间态；③radio-cards 首帧无布局跳位（900px 与 125% 逐帧包围盒一致）；④RTL 方向继承与横向键盘反转实测通过；⑤两项 WASM 构建、`kui add` 独立安装（原生 + wasm32）与网页实测通过；⑥`color_motion` 抽取后 ai-spark 强制重编与安装依赖回归通过。M（同速官网对照）与 P（桌面原生／真触摸／Safari）仍缺，故仍未打勾。

**当前用户要求是先整理审查文档。后续 AI 先按本表核对、记录差异；收到实现任务后再修改代码。不得因为读了本表就自动启动批量修改、构建、录屏或推送。**

## 目录

- [范围与旧结论纠正](#范围与旧结论纠正)
- [本轮验收门槛](#本轮验收门槛)
- [两项优先返查](#两项优先返查)
- [输入、选择与手势控件：12 项](#inputs)
- [导航、消息、表格、浮层与进度：12 项](#surfaces)
- [图表、指标与活动：17 项](#charts)
- [文字与媒体：13 项](#media)
- [AI 任务、编辑与宿主交互：21 项](#workflows)
- [AI 状态与过程示意：10 项](#scenarios)
- [每项必须留下的证据](#每项必须留下的证据)
- [给下一位 AI 的任务](#给下一位-ai-的任务)
- [原通过记录的处理](#原通过记录的处理)

## 范围与旧结论纠正

- 当前本地清单为 **193 项**；其中 **108 项**按用户要求默认通过、排除本轮重审，**其余 85 项全部进入本表**。划分按历史记录中的 `user_default_pass`，不凭日期或组件分类猜测。基线 Git：`6c9a53ebfa16aa59c4ab388f18cbb944e8ea6594`。
- 历史数据位于[component-audit-status.json](</Users/leon/Documents/Kense ui/KUI/docs/audits/component-audit-status.json>)，保留原记录作为证据索引。旧轮 85 项均记录过源码、原生/WASM 和安装检查，其中 16 项记录过视觉/动效通过；**这些旧勾选不等于本轮已通过，不得自动带入新表**。
- Activity rings、Sankey flow 的旧记录只证明本地浅/暗/窄屏等状态可渲染，以及约 6 帧入场有变化；没有记录同场景官网实物对照。文档还写着逐帧对比或输入验证待办，状态却被标为通过。这不足以支持“和官网一致”的结论。
- 本轮撤回新增组件的整体对齐通过结论；原始日志保留。原生/WASM/安装的旧证据可在确认源码版本、依赖和预览版本一致后复用，但必须逐项引用，不能把“能编译”“能点击”“首尾截图相似”合并为视觉、动效通过。
- 已修过的胶囊形状、刻度/价格读数等专项问题，保留它们的修复记录；专项通过不代表整个组件已完成官网体验对齐。
- 本表的“核对/审查”是待办，不预先断言这些地方都有 bug。“文档承认”或“用户指出”的问题会单独注明。审查发现实现其实已正确，应附证据通过，不必为重审而重写。
- 本轮以此 Markdown 为当前结论。旧 JSON、旧统计脚本及旧“已通过”文案属于上一轮；不得再用旧脚本把它们覆盖回桌面的重审表。

## 本轮验收门槛

| 阶段 | AI 必须核对什么 | 通过证据 |
| --- | --- | --- |
| S 源码/参考 | 从官方索引定位真实组件及全部示例；读取可合法访问的 TSX/CSS、Token、motion 配置和公开文档。记录来源地址、日期、版本/哈希。Pro 源码不可访问时明确写“只有公开文档/实物”，不得写成已读源码。 | 来源清单和逐参数依据；无法核实的值标记未核实。 |
| A Rust API | 做“参考属性/默认值 → KUI builder/enum/事件/宿主适配”的等效映射，查范围限制、格式化、重复/过期请求、取消、回调时机；审已有 Rust 源码和安装后的源码，不把 React 属性照搬成虚假 Rust API。 | 参数映射表、真实可编译用法、差异说明。 |
| V 静态视觉 | 同数据、同示例、同窗口、缩放、主题和字体比官网与本地；量尺寸、布局、色值、描边、图标、字形/基线、阴影、圆角、裁切。覆盖默认/悬停/按压/焦点/选中/禁用及适用的加载/空/错/成功态。 | 同尺寸并排截图及测量。几何目标误差≤1 CSS px；字体渲染差异单列，不借此放过布局错误。 |
| I 交互/恢复 | 按每行专属场景核对鼠标、键盘、触摸、失焦/取消、重复操作、错误恢复、受控宿主和真实副作用。 | 可重复步骤、实际观察、有效的行为测试；未测试的真机路径明确保留。 |
| M 连续动效 | 同一输入时刻起播，检查触发、运动中和收尾；核对时长、缓动/弹簧、错峰、起止值、几何/颜色/文字同步，测试约65ms快速反切和中途换目标。不能用6张任意时刻的本地截图替代官网对照。 | 同速可播放并排录像或足够连续的时间标记帧；60Hz下主要起止目标误差≤2帧。测不到就留待验，不许写已达标。 |
| P 平台/可访问性 | 同份 Rust + GPUI 在桌面和 Web 验证；Safari、亮/暗、320px、100%/125%显示比例、减少动效、RTL适用场景。查焦点返回、语义/名称、状态播报、IME与触摸滚动冲突。 | 环境、输入和结果逐项记录；浏览器模拟触摸不自动代表真实设备已通过。 |
| D 分发/文档 | 本地预览、安装项、文档/示例使用同份源码；核对注册表依赖和哈希。新空白项目安装并原生/WASM编译，或引用源码版本相同的有效记录。 | 安装命令、项目/构建记录、版本、预览哈希；中英文页面和链接核对。 |
| F 性能/生命周期 | 可见动画是否接近16.7ms帧预算，频繁交互有无长帧；离屏、暂停、静止不空转；异步/订阅/媒体释放。导航不重启画布、主题不闪白。 | 场景与采样方法、长帧/空转观察。只跑一个重任务，不启动多浏览器/批量录屏；不把截图流畅写成恒定60fps。 |

允许保留的产品差异只有已明确指定的 KUI 品牌、中文本地化、Rust/GPUI 与宿主能力边界，以及 DESIGN.md 的形状规则：普通矩形默认8px；Switch/BillingToggle/GlassSegmented等明确的胶囊和圆形保留原形态。其他省略、近似和功能缺口必须登记，不能用“原创实现”解释为可以随意换体验。

表中状态顺序为 **S / A / V / I / M / P / D / F**。本轮初始均为 `□□□□□□□□`。完成一个阶段，附证据后改对应格为 `✓`；八项齐全才把该行改成“通过”。不能把本表当作85项都已有同样缺陷。

## 两项优先返查

- **P0 [Activity rings](#review-activity-rings)**：用户已指出整体差异，先完整核对视觉结构和状态交互，不先微调时长掩盖几何差异。
- **P0 [Sankey flow](#review-sankey-flow)**：用户已指出整体差异，先核对数据与布局、流带和节点，再核对高亮、粒子、提示及数据切换。
- P1 为高频交互、文档已知缺口、已有用户专项修复或高风险连续状态；P2 是其他待核对项。优先级只是顺序，不是质量结论。

<a id="inputs"></a>

## 输入、选择与手势控件（12 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-rating"></a>01 [评分](http://127.0.0.1:4321/kui/components/rating/)<br>[rating](</Users/leon/Documents/Kense ui/KUI/docs/items/rating.md>) | P1 | 星形轮廓、半星裁切、Sm/Md/Lg尺寸、标签与读数占位。 | 悬停恢复、跨星拖选、清除/受控确认；逐星错峰、按压回弹、词级换字和说明面板中间帧。 | 本轮已按官方 TSX/CSS 核对几何（cell 36/glyph 24/gap 2 → 星距 38，与本地实测一致）、交互与键盘全路径、变词模糊（`blur.soft 4px`/`blur.subtle 2px` 与官方 token 逐项吻合，实测到双词共存中间帧）、说明面板退出交叉淡出（拍到旧行降透明度中间态）、快速改分取消旧计时（单测）、独立安装与文档同步，见下方记录；M（同速官网对照）与 P（桌面原生/真触摸/Safari）仍待验。 | 部分通过 · `SAVI□□D□` |
| <a id="review-billing-toggle"></a>02 [计费周期](http://127.0.0.1:4321/kui/components/billing-toggle/)<br>[billing-toggle](</Users/leon/Documents/Kense ui/KUI/docs/items/billing-toggle.md>) | P1 | 胶囊轨道/高亮/优惠标记、间隙与中英文宽度。 | 切换月份/年度、重复选择、方向键；高亮中途反向、阴影与文字同步。 | 已做胶囊修复；需用本轮证据核对完整体验。 | 待重审 · `□□□□□□□□` |
| <a id="review-toggle-group"></a>03 [切换组](http://127.0.0.1:4321/kui/components/toggle-group/)<br>[toggle-group](</Users/leon/Documents/Kense ui/KUI/docs/items/toggle-group.md>) | P2 | 单选/多选组尺寸、首尾圆角、分隔线、选中和悬停重合。 | 受控值、禁用跳过、方向键；单选高亮滑移与多选独立填充，快速连点。 | 旧有本地测试记录；未作为本轮官网一致性结论。 | 待重审 · `□□□□□□□□` |
| <a id="review-radio-cards"></a>04 [选项卡片](http://127.0.0.1:4321/kui/components/radio-cards/)<br>[radio-cards](</Users/leon/Documents/Kense ui/KUI/docs/items/radio-cards.md>) | P1 | 各卡片布局、选中标记、图标/标题/说明基线、描边。 | 单选、受控拒绝、禁用、键盘与触摸；标记切换和卡片反馈。 | 本轮已按官方 TSX/CSS 逐参数核对（grid/list 布局、padding 14/16 与 13/16/13/14、border 1px、ring 1.5px、indicator 18/dot 6、icon 18、meta 18、0.16s 均吻合）、首帧无跳变（900px 与 125% 墨迹包围盒逐帧一致）、RTL 网格镜像与横向键反转运行时实测通过、独立安装（原生+wasm32）与文档同步，见下方记录；官方圆角 20px 与 KUI 8px 属登记的产品差异。M（同速官网对照）与 P（桌面原生/真触摸/Safari）仍待验。 | 部分通过 · `SAVI□□D□` |
| <a id="review-money-input"></a>05 [金额输入](http://127.0.0.1:4321/kui/components/money-input/)<br>[money-input](</Users/leon/Documents/Kense ui/KUI/docs/items/money-input.md>) | P1 | 币种/金额/快捷增额布局、字宽、小数点和光标位置。 | 输入提交/失焦、非法草稿回退、上下界拒绝、快捷步进；边界拉伸和提示闪动。 | 已有步进拒绝专项修复；仍需完整源码及实物对照。 | 待重审 · `□□□□□□□□` |
| <a id="review-elastic-slider"></a>06 [弹性滑块](http://127.0.0.1:4321/kui/components/elastic-slider/)<br>[elastic-slider](</Users/leon/Documents/Kense ui/KUI/docs/items/elastic-slider.md>) | P1 | 轨道/滑块/气泡尺寸、数值和边界位置。 | 拖动/键盘/触控、过界拉伸、回弹和速度继承；滚动意图、快速反向。 | 旧入场/拖动记录不替代与官网同速轨迹对照。 | 待重审 · `□□□□□□□□` |
| <a id="review-range-histogram-slider"></a>07 [区间分布滑块](http://127.0.0.1:4321/kui/components/range-histogram-slider/)<br>[range-histogram-slider](</Users/leon/Documents/Kense ui/KUI/docs/items/range-histogram-slider.md>) | P1 | 柱形分桶、两端滑块、边界高亮、右对齐等宽区间读数。 | 双端不交叉、柱形点击、分桶吸附、计数；拖动/松手/快速往返及位数变化。 | 读数抖动已专项修复并有回归；全组件仍须重审。 | 待重审 · `□□□□□□□□` |
| <a id="review-ruler-slider"></a>08 [刻度滑块](http://127.0.0.1:4321/kui/components/ruler-slider/)<br>[ruler-slider](</Users/leon/Documents/Kense ui/KUI/docs/items/ruler-slider.md>) | P1 | 刻度间距、固定指针、边缘淡化、读数/单位基线。 | 半步固定小数、99.5→100.0进位、拖甩惯性、横向滚轮、反向与到边；数字列持续性。 | 读数跳位已专项修复；还需全轨道/惯性和官网连续帧对照。 | 待重审 · `□□□□□□□□` |
| <a id="review-signature-pad"></a>09 [签名板](http://127.0.0.1:4321/kui/components/signature-pad/)<br>[signature-pad](</Users/leon/Documents/Kense ui/KUI/docs/items/signature-pad.md>) | P1 | 画布比例、笔触端头/粗细、工具栏、空/已有签名状态。 | 鼠标/触摸/压力、撤销重做/清空/回放/导出、触摸取消；笔迹延迟和回放顺序。 | 确认导出的实际内容和宿主保存边界，不能只点击按钮。 | 待重审 · `□□□□□□□□` |
| <a id="review-scrub-input"></a>10 [拖动数值输入](http://127.0.0.1:4321/kui/components/scrub-input/)<br>[scrub-input](</Users/leon/Documents/Kense ui/KUI/docs/items/scrub-input.md>) | P1 | 数值/单位/拖动标签、编辑与非编辑态宽度、光标。 | Shift/Alt步进、表达式/单位切换、边界/环绕、拖动取消；数值稳定与弹性反馈。 | 需审当前用户正在看的实际示例与连续操作。 | 待重审 · `□□□□□□□□` |
| <a id="review-morph-select"></a>11 [形变选择器](http://127.0.0.1:4321/kui/components/morph-select/)<br>[morph-select](</Users/leon/Documents/Kense ui/KUI/docs/items/morph-select.md>) | P1 | 收起键→搜索面板的同一表面、分组行、图标、选中高亮。 | 搜索/禁用/空结果/Page键、选择/取消/焦点返回；宽高、换词、快速反开合与防裁切。 | 既有功能记录不足以证明形变中间过程一致。 | 待重审 · `□□□□□□□□` |
| <a id="review-glass-segmented"></a>12 [玻璃分段选择](http://127.0.0.1:4321/kui/components/glass-segmented/)<br>[glass-segmented](</Users/leon/Documents/Kense ui/KUI/docs/items/glass-segmented.md>) | P1 | 胶囊外轨道/玻璃底片、描边、阴影、间隙和标签。 | 拖拽/抛掷/方向键/禁用；前后沿独立弹簧、拉伸、回弹和中断。 | 胶囊已恢复；玻璃层次及不同输入路径仍需完整比对。 | 待重审 · `□□□□□□□□` |

<a id="surfaces"></a>

## 导航、消息、表格、浮层与进度（12 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-countdown"></a>13 [倒计时](http://127.0.0.1:4321/kui/components/countdown/)<br>[countdown](</Users/leon/Documents/Kense ui/KUI/docs/items/countdown.md>) | P1 | 各数字列、分隔符、单位、完成文案与宽度。 | 暂停/重置、59→00借位、到零只触发一次；最后一秒到完成态及减少动效。 | 文档明确完成过渡未采样，结束态为硬切；必须单列待补。 | 待重审 · `□□□□□□□□` |
| <a id="review-announcement-bar"></a>14 [公告栏](http://127.0.0.1:4321/kui/components/announcement-bar/)<br>[announcement-bar](</Users/leon/Documents/Kense ui/KUI/docs/items/announcement-bar.md>) | P2 | 公告高度、图标/操作/关闭键位置、窄屏换行。 | 7秒轮播是否与源码一致、hover/focus暂停、关闭与重开；消息交叉淡入和高度收起。 | 需覆盖多个公告及关闭后的布局，不以首帧为准。 | 待重审 · `□□□□□□□□` |
| <a id="review-cookie-consent"></a>15 [Cookie 同意设置](http://127.0.0.1:4321/kui/components/cookie-consent/)<br>[cookie-consent](</Users/leon/Documents/Kense ui/KUI/docs/items/cookie-consent.md>) | P1 | 通知/偏好设置/保存后入口的尺寸、分组、胶囊开关与按钮。 | 接受/拒绝/自定义、必选禁用、取消保留策略、持久化宿主；表面展开回收及焦点。 | 复用Switch的胶囊修复有效；其余流程和动效待对照。 | 待重审 · `□□□□□□□□` |
| <a id="review-undo-toast"></a>16 [撤销提示](http://127.0.0.1:4321/kui/components/undo-toast/)<br>[undo-toast](</Users/leon/Documents/Kense ui/KUI/docs/items/undo-toast.md>) | P1 | 被移除行、通知、倒计时与撤销/关闭键位置。 | 合并批次、超时提交、hover/focus暂停、撤销后数据/焦点恢复；行收展与成功反馈。 | 要检查超时和多次删除，不只演示一次撤销。 | 待重审 · `□□□□□□□□` |
| <a id="review-tree-table"></a>17 [树形表格](http://127.0.0.1:4321/kui/components/tree-table/)<br>[tree-table](</Users/leon/Documents/Kense ui/KUI/docs/items/tree-table.md>) | P1 | 列宽、树缩进、连接线/展开键、选中/半选框和加载行。 | 展开全选、层级键盘、懒加载失败重试、排序与稳定ID；行高展开和滚动定位。 | 核对错误行是否能真实恢复，避免仅增加提示文字。 | 待重审 · `□□□□□□□□` |
| <a id="review-mega-menu-morph"></a>18 [形变导航菜单](http://127.0.0.1:4321/kui/components/mega-menu-morph/)<br>[mega-menu-morph](</Users/leon/Documents/Kense ui/KUI/docs/items/mega-menu-morph.md>) | P1 | 导航锚点、单一浮层的宽高、面板内布局与窄屏折叠。 | hover意图/跨栏目/Esc/焦点；共享面板移动、交叉淡入、快速反向和展开边界。 | 必须查面板移动途中，不能只比两个静态菜单。 | 待重审 · `□□□□□□□□` |
| <a id="review-glass-menu"></a>19 [玻璃操作菜单](http://127.0.0.1:4321/kui/components/glass-menu/)<br>[glass-menu](</Users/leon/Documents/Kense ui/KUI/docs/items/glass-menu.md>) | P1 | 触发器到菜单的形变、玻璃层/行高/图标/快捷键与子菜单。 | 子层前进返回、禁用/勾选/Esc/焦点；层间水平切换、行错峰及快速开关。 | 核对源码参数与可实现的玻璃效果差异，不以透明底替代。 | 待重审 · `□□□□□□□□` |
| <a id="review-presence-stack"></a>20 [在线成员](http://127.0.0.1:4321/kui/components/presence-stack/)<br>[presence-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/presence-stack.md>) | P1 | 头像叠放/状态点/溢出计数、展开名单、名字提示的对齐。 | hover/focus扇出、键盘点名、选人/关闭；稳定ID进出、头像位置、文字宽度与输入点。 | 旧全仓测试曾有成员选择器不匹配记录；需用当前版本重新验证。 | 待重审 · `□□□□□□□□` |
| <a id="review-toast-stack"></a>21 [通知堆叠](http://127.0.0.1:4321/kui/components/toast-stack/)<br>[toast-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/toast-stack.md>) | P1 | 前后卡露出、深度/阴影/位置、正文/操作/关闭。 | 连续推入/更新/删除、hover/focus暂停与展开、超时；布局弹簧、堆叠前移和焦点。 | 覆盖至少4条与不同高度，检查裁切及退出后正文滚动。 | 待重审 · `□□□□□□□□` |
| <a id="review-bottom-sheet"></a>22 [底部面板](http://127.0.0.1:4321/kui/components/bottom-sheet/)<br>[bottom-sheet](</Users/leon/Documents/Kense ui/KUI/docs/items/bottom-sheet.md>) | P1 | 遮罩/面板/手柄、安全区、内容区与分档高度。 | 拖动/甩动/键盘、内外滚动冲突、边界取消、关闭焦点；阻尼/投射/吸附和减少动效。 | 真实触摸与Safari需单独实测，模拟截图不算真机完成。 | 待重审 · `□□□□□□□□` |
| <a id="review-sheet-stack"></a>23 [多层面板](http://127.0.0.1:4321/kui/components/sheet-stack/)<br>[sheet-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/sheet-stack.md>) | P1 | 前后层尺寸/退距、遮罩叠加、标题栏和窄屏边界。 | push/pop/Esc/拖回、焦点层级、取消与反开；背景层深度、顶层跟手和连续返回。 | 核对只关闭顶层、滚动/命中不穿透，非两张卡静态重叠。 | 待重审 · `□□□□□□□□` |
| <a id="review-morph-loader"></a>24 [形变加载](http://127.0.0.1:4321/kui/components/morph-loader/)<br>[morph-loader](</Users/leon/Documents/Kense ui/KUI/docs/items/morph-loader.md>) | P1 | dots/bars/ring/square与成功/失败的几何、笔画、固定足迹。 | 快速换形/重复结果、暂停/隐藏；四笔独立轨迹、速度接续、收拢/回弹和最终静止。 | 已有较详细参数与局部修复；仍需所有形状同速官方对照，不能只验一个加载态。 | 待重审 · `□□□□□□□□` |

<a id="charts"></a>

## 图表、指标与活动（17 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-gauge"></a>25 [仪表盘](http://127.0.0.1:4321/kui/components/gauge/)<br>[gauge](</Users/leon/Documents/Kense ui/KUI/docs/items/gauge.md>) | P1 | 起止角/弧宽/端帽/轨道、中心数字、刻度和配色。 | 空/极值/越界/非有限值；弧与读数同步、快速更新、减少动效。 | 旧通过只显示弧和数字可渲染；需官网同场景比较。 | 待重审 · `□□□□□□□□` |
| <a id="review-animated-counter"></a>26 [动态数字](http://127.0.0.1:4321/kui/components/animated-counter/)<br>[animated-counter](</Users/leon/Documents/Kense ui/KUI/docs/items/animated-counter.md>) | P1 | 等宽数字、分组符/正负号/小数、前后缀和基线。 | 进位/借位/跨零、长短数值、快速反切、格式更新；旧列退出/新列展开和静止帧。 | 复用读数不能在动画结束时换排版；需像素与几何证据。 | 待重审 · `□□□□□□□□` |
| <a id="review-waffle-chart"></a>27 [华夫图](http://127.0.0.1:4321/kui/components/waffle-chart/)<br>[waffle-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/waffle-chart.md>) | P2 | 单元数量/间距/圆角、类别分配、图例、读数与配色。 | 图例筛选至少保留一类、键盘定位；单元稳定ID重排、交叉顺序和入场错峰。 | 核对百分比到格数的舍入与最终总数。 | 待重审 · `□□□□□□□□` |
| <a id="review-slope-chart"></a>28 [斜率图](http://127.0.0.1:4321/kui/components/slope-chart/)<br>[slope-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/slope-chart.md>) | P2 | 左右端点、线宽、标签避让/基线、刻度和图例。 | 悬停/选择/键盘、空值/新旧系列；端点与路径插值、淡化和提示跟随。 | 检查交叉线和拥挤标签，不仅看稀疏样例。 | 待重审 · `□□□□□□□□` |
| <a id="review-radar-chart"></a>29 [雷达图](http://127.0.0.1:4321/kui/components/radar-chart/)<br>[radar-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/radar-chart.md>) | P2 | 网格/轴数/标签方向、填充透明度、点和图例。 | 不同系列/极值/缺轴、选中/禁用；顶点/轮廓连续变形和提示位置。 | 核对当前0–100、最多12轴等限制与参考API，写出等效映射。 | 待重审 · `□□□□□□□□` |
| <a id="review-split-bar"></a>30 [占比条](http://127.0.0.1:4321/kui/components/split-bar/)<br>[split-bar](</Users/leon/Documents/Kense ui/KUI/docs/items/split-bar.md>) | P1 | 分段比例/间隙/端帽、颜色、内部标签和图例。 | hover/tap锁定、极小段、零值/总和变化；选中段展开、其他段让位、字宽拟合。 | 检查移动中分段边界与命中区一致，不能越出外轮廓。 | 待重审 · `□□□□□□□□` |
| <a id="review-race-bar-chart"></a>31 [动态排名条形图](http://127.0.0.1:4321/kui/components/race-bar-chart/)<br>[race-bar-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/race-bar-chart.md>) | P1 | 排名/条宽/标签/数值轴、播放栏与时间标记。 | 播放暂停/重启/跳帧、并列名次、进出榜；ID稳定、排序换位和数值滚动。 | 该文件此前存在其他Agent并行编辑；审查前核对当前版本，勿覆盖他人修改。 | 待重审 · `□□□□□□□□` |
| <a id="review-funnel-chart"></a>32 [转化漏斗](http://127.0.0.1:4321/kui/components/funnel-chart/)<br>[funnel-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/funnel-chart.md>) | P1 | 各级梯形/带宽、接缝/曲率、转化数字、图例和颜色。 | 切换阶段/系列、异常与零值；整体轮廓插值、流动粒子/高亮及离屏暂停。 | 核对真实比例、衰减和提示内容，不以任意多边形替代。 | 待重审 · `□□□□□□□□` |
| <a id="review-streamgraph"></a>33 [流图](http://127.0.0.1:4321/kui/components/streamgraph/)<br>[streamgraph](</Users/leon/Documents/Kense ui/KUI/docs/items/streamgraph.md>) | P2 | 堆叠基线、曲线平滑、颜色、坐标/图例及边缘留白。 | 筛选/数据范围/悬停、空或缺失值；共享网格插值、路径/提示与快速更新。 | 核对面积与原数据一致、无负厚度或交叉。 | 待重审 · `□□□□□□□□` |
| <a id="review-treemap"></a>34 [矩形树图](http://127.0.0.1:4321/kui/components/treemap/)<br>[treemap](</Users/leon/Documents/Kense ui/KUI/docs/items/treemap.md>) | P1 | 面积/分割算法、层级内边距、圆角、标签阈值和颜色。 | 钻入/返回/面包屑/指标切换、空层级；缩放原点、稳定ID重排、快速返回。 | 旧本地四态不等于官方分割和缩放一致；需同数据集比较。 | 待重审 · `□□□□□□□□` |
| <a id="review-brush-chart"></a>35 [区间趋势图](http://127.0.0.1:4321/kui/components/brush-chart/)<br>[brush-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/brush-chart.md>) | P1 | 主图/缩略图、选区/手柄、坐标与日期读数。 | 拖选/拖边/整体移动、键盘、最小区间、边界；范围/坐标弹簧与提示更新。 | 检查拖动时不被正文抢滚、松手后区间与图表一致。 | 待重审 · `□□□□□□□□` |
| <a id="review-ridgeline"></a>36 [山脊分布图](http://127.0.0.1:4321/kui/components/ridgeline/)<br>[ridgeline](</Users/leon/Documents/Kense ui/KUI/docs/items/ridgeline.md>) | P2 | 各分布曲线/垂直间距、轴/标签、填充与前后遮挡。 | 分布切换、峰值/空样本、hover/选择；抬升、曲线插值和提示跟随。 | 核对密度/尺度含义，不仅确认有一排曲线。 | 待重审 · `□□□□□□□□` |
| <a id="review-realtime-stream"></a>37 [实时数据流](http://127.0.0.1:4321/kui/components/realtime-stream/)<br>[realtime-stream](</Users/leon/Documents/Kense ui/KUI/docs/items/realtime-stream.md>) | P1 | 时间窗/坐标、曲线/面积、最新值与控制栏。 | 暂停/恢复/重连、窗口滑动、缺点/乱序/空数据；路径平移、点入场和离屏时钟。 | 验证暂停恢复不跳回旧时间；模拟数据须明确标注。 | 待重审 · `□□□□□□□□` |
| <a id="review-sunburst"></a>38 [旭日图](http://127.0.0.1:4321/kui/components/sunburst/)<br>[sunburst](</Users/leon/Documents/Kense ui/KUI/docs/items/sunburst.md>) | P1 | 每层内外半径、角度/间隙、弧标签方向、中心数字和面包屑。 | 钻入/退回/键盘、稳定ID、非法值；弧域重映射、中心读数和快速反切。 | 文档明确无障碍表未用屏幕阅读器验证。 | 待重审 · `□□□□□□□□` |
| <a id="review-sankey-flow"></a>39 [桑基流向图](http://127.0.0.1:4321/kui/components/sankey-flow/)<br>[sankey-flow](</Users/leon/Documents/Kense ui/KUI/docs/items/sankey-flow.md>) | P0 | 先同官网核对整体布局：节点/列位置、带宽比例、留白、颜色/透明度、标题和标签。 | 悬停上游/下游追踪、流带命中、提示/固定、周期切换；带形变、粒子密度/速度与连续中间帧。 | 用户指出完全不同。旧记录只有本地四态与6帧入场；文档仍称逐帧对照待办，撤回旧视觉/动效通过。**本轮已核对**：缺官网 meta 汇总行(`11,200 signups, Jul to Sep 2026`)、列头位置不同、未聚焦流带为单色灰(应为系列色)、粒子 12 vs 官方 34、首帧 0.95s/切换 0.34s vs 官方 0.4s/0.42s。 | 部分通过 · `SAV□□□□□` |
| <a id="review-metric-ticker"></a>40 [实时指标条](http://127.0.0.1:4321/kui/components/metric-ticker/)<br>[metric-ticker](</Users/leon/Documents/Kense ui/KUI/docs/items/metric-ticker.md>) | P1 | 指标列宽、数值/单位/涨跌符号、图标、分隔和趋势小图。 | 数据刷新/排序、长短数值与缺失项；等宽数字进位、涨跌变色、重新排列与离屏暂停。 | 文档有未验证边界，需查真实刷新而非首屏静态数据。 | 待重审 · `□□□□□□□□` |
| <a id="review-activity-rings"></a>41 [活动圆环](http://127.0.0.1:4321/kui/components/activity-rings/)<br>[activity-rings](</Users/leon/Documents/Kense ui/KUI/docs/items/activity-rings.md>) | P0 | 先确认官网是连续弧还是刻线表现；核对环数/半径/厚度/间距、端帽/起点、颜色渐变、日期栏和图例。 | 切换日期、hover/tap选环、中心读数、达标与超额；入场错峰、跨100%接续、数值/环同步和快速反切。 | 用户指出差异很大。旧记录只有本地四态与6帧扫入，触摸/焦点/播报仍待验，撤回旧视觉/动效通过。**本轮已核对**：官网为**刻线环**，半径/刻度数/描边宽/基色/视口与本地一致；差异在第二圈应内移(89/69/49 而非 mid)、缺常显中心读数(`0% of goals`)、缺 `marker` 指针、日期按钮缺 30×30 三道 mini 环、默认选中日应为最后一天。 | 部分通过 · `SAV□□□□□` |

<a id="media"></a>

## 文字与媒体（13 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-text-reveal"></a>42 [文字揭示](http://127.0.0.1:4321/kui/components/text-reveal/)<br>[text-reveal](</Users/leon/Documents/Kense ui/KUI/docs/items/text-reveal.md>) | P2 | 词/行分段、换行、字号/字距和最终基线。 | 延迟/重播/换文本；逐词错峰、位移/透明度/模糊、收尾和减少动效。 | 查真实多行、中英混排与快速更新，不只录短单词。 | 待重审 · `□□□□□□□□` |
| <a id="review-in-view-title"></a>43 [视区标题](http://127.0.0.1:4321/kui/components/in-view-title/)<br>[in-view-title](</Users/leon/Documents/Kense ui/KUI/docs/items/in-view-title.md>) | P2 | 标题分行、溢出裁切、起止位置和页面留白。 | 滚入/滚出/返回、once与重播；触发阈值、延迟和入场轨迹。 | 需用实际页面滚动触发，不能仅重新加载录入场。 | 待重审 · `□□□□□□□□` |
| <a id="review-text-morph"></a>44 [文字形变](http://127.0.0.1:4321/kui/components/text-morph/)<br>[text-morph](</Users/leon/Documents/Kense ui/KUI/docs/items/text-morph.md>) | P1 | 相同/变化字符定位、字号/字距、宽度和基线。 | 长短句/中英文/多行、快速反切；匹配字符移动、新旧层交叉淡入、最终布局稳定。 | 查动画结束是否切换排版，防止重复此前数字抖动。 | 待重审 · `□□□□□□□□` |
| <a id="review-text-shimmer"></a>45 [文字微光](http://127.0.0.1:4321/kui/components/text-shimmer/)<br>[text-shimmer](</Users/leon/Documents/Kense ui/KUI/docs/items/text-shimmer.md>) | P2 | 亮带宽度、底色/高亮色、文字边缘和裁剪。 | 开始/暂停/恢复/隐藏；扫过方向、周期、速度、相位连续和减少动效。 | 核对是否真正作用于字形，而非整块背景渐变。 | 待重审 · `□□□□□□□□` |
| <a id="review-slot-text"></a>46 [数字滚轮文字](http://127.0.0.1:4321/kui/components/slot-text/)<br>[slot-text](</Users/leon/Documents/Kense ui/KUI/docs/items/slot-text.md>) | P1 | 数字列宽、符号/小数/分组符、单位和基线。 | 进位/借位/跨零、快速更新和长度变化；滚轮路线、错峰与收尾。 | 必须查静止帧与运动帧同一排版，不能只判断数字最终正确。 | 待重审 · `□□□□□□□□` |
| <a id="review-image-compare"></a>47 [图片对比](http://127.0.0.1:4321/kui/components/image-compare/)<br>[image-compare](</Users/leon/Documents/Kense ui/KUI/docs/items/image-compare.md>) | P1 | 图片缩放/对齐、分隔线/手柄、标签、容器比例。 | 鼠标/触摸拖动、键盘/Home/End、容器缩放；直接跟手与键盘弹簧、边界命中。 | 检查纵向手势滚正文、图像不因切边移动或失真。 | 待重审 · `□□□□□□□□` |
| <a id="review-carousel"></a>48 [图片轮播](http://127.0.0.1:4321/kui/components/carousel/)<br>[carousel](</Users/leon/Documents/Kense ui/KUI/docs/items/carousel.md>) | P1 | 图片裁切/间距、邻页露出、分页/箭头及图注。 | 拖拽/抛掷/键盘、首尾/循环、尺寸变化；捕获手势、吸附/回弹、自动播放暂停。 | 官网有的示例和交互逐一对照，不能只实现左右换图。 | 待重审 · `□□□□□□□□` |
| <a id="review-card-stack"></a>49 [卡片堆叠](http://127.0.0.1:4321/kui/components/card-stack/)<br>[card-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/card-stack.md>) | P1 | 层叠位移/缩放/旋转、卡片阴影、图像和决策按钮。 | 拖动阈值、方向判定、取消、撤销、用尽；顶卡出场、后层前进及快速连续操作。 | 核对中间形态与命中区域，不以硬删除卡片替代。 | 待重审 · `□□□□□□□□` |
| <a id="review-product-gallery"></a>50 [商品图集](http://127.0.0.1:4321/kui/components/product-gallery/)<br>[product-gallery](</Users/leon/Documents/Kense ui/KUI/docs/items/product-gallery.md>) | P1 | 主图/缩略图、颜色/尺码、价格/按钮、放大镜与查看器。 | 换图/缺图、可用尺码、缩放/关闭/焦点；图片淡换、放大镜跟随和布局稳定。 | 核对实际资源、禁用组合及窄屏溢出。 | 待重审 · `□□□□□□□□` |
| <a id="review-voice-orb"></a>51 [语音球](http://127.0.0.1:4321/kui/components/voice-orb/)<br>[voice-orb](</Users/leon/Documents/Kense ui/KUI/docs/items/voice-orb.md>) | P1 | 球体轮廓、材质/渐变/光晕、各状态颜色和周边控件。 | 空闲/听取/思考/播报/错误/暂停；音量驱动形变、过渡与隐藏时钟。 | 现有文档过于概括；需逐状态记录参数，分清样例与宿主音频。 | 待重审 · `□□□□□□□□` |
| <a id="review-now-playing"></a>52 [正在播放](http://127.0.0.1:4321/kui/components/now-playing/)<br>[now-playing](</Users/leon/Documents/Kense ui/KUI/docs/items/now-playing.md>) | P1 | 封面/标题/时间轴、播放键、音量和队列。 | 播放暂停/拖进度/切曲、加载/错误、时长变化；封面/文本切换、波形与进度同步。 | 不得只切换播放图标；核对音频宿主确认和实际时间推进。 | 待重审 · `□□□□□□□□` |
| <a id="review-voice-recorder"></a>53 [语音录音](http://127.0.0.1:4321/kui/components/voice-recorder/)<br>[voice-recorder](</Users/leon/Documents/Kense ui/KUI/docs/items/voice-recorder.md>) | P1 | 录音/回听/草稿/发送布局、计时、波形和状态键。 | 权限/取消/重录/回听/发送/失败重试；设备释放、草稿保留、波形/计时连续。 | 文档仅描述通用宿主边界，需补具体状态与真实/样例适配验收。 | 待重审 · `□□□□□□□□` |
| <a id="review-image-reveal"></a>54 [图片生成显现](http://127.0.0.1:4321/kui/components/image-reveal/)<br>[image-reveal](</Users/leon/Documents/Kense ui/KUI/docs/items/image-reveal.md>) | P1 | 预览/成图比例、揭示边缘、模糊/颗粒/遮罩和进度标记。 | 开始/取消/失败/重试/旧任务回执、替换图片；软边推进、清晰化及完成收尾。 | 文档明确模糊、颗粒、软边未实现或未视觉验证，不能用硬切遮罩当作完成。 | 待重审 · `□□□□□□□□` |

<a id="workflows"></a>

## AI 任务、编辑与宿主交互（21 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-tldr-toggle"></a>55 [摘要切换](http://127.0.0.1:4321/kui/components/tldr-toggle/)<br>[tldr-toggle](</Users/leon/Documents/Kense ui/KUI/docs/items/tldr-toggle.md>) | P1 | 摘要/原文/长度控制、内容卡高度和来源高亮。 | 生成/停止/重试、编辑时的结果处理、引用定位；切换高度、入场和高亮退场。 | 核对整段生成流程，不能把本地样例替换当作真实宿主完成。 | 待重审 · `□□□□□□□□` |
| <a id="review-smart-form-fill"></a>56 [智能表单填写](http://127.0.0.1:4321/kui/components/smart-form-fill/)<br>[smart-form-fill](</Users/leon/Documents/Kense ui/KUI/docs/items/smart-form-fill.md>) | P1 | 原文区、建议字段、置信度标记、审核与保存位置。 | 低置信度审核、编辑冲突、结果排队/接受/拒绝、停止重试；来源高亮和逐字段填入。 | 需区分样例解析与宿主接口；验证已有编辑不会被晚到结果覆盖。 | 待重审 · `□□□□□□□□` |
| <a id="review-enrich-column"></a>57 [数据补全列](http://127.0.0.1:4321/kui/components/enrich-column/)<br>[enrich-column](</Users/leon/Documents/Kense ui/KUI/docs/items/enrich-column.md>) | P1 | 指令编辑器、表格行列、待处理/完成/失败单元格。 | 单行/全部运行、停止/重试、编辑冲突与请求序号；逐格反馈、编辑器高度和焦点。 | 验证乱序/过期结果及重试不重复应用。 | 待重审 · `□□□□□□□□` |
| <a id="review-ticket-router"></a>58 [工单分流](http://127.0.0.1:4321/kui/components/ticket-router/)<br>[ticket-router](</Users/leon/Documents/Kense ui/KUI/docs/items/ticket-router.md>) | P1 | 工单卡、分类/目标队列、低置信度审核区、操作位置。 | 启动/暂停、人工改派、失败重试、宿主确认；工单跨状态移动与退回。 | 指派必须有宿主确认，审查是否只切换了本地标签。 | 待重审 · `□□□□□□□□` |
| <a id="review-reply-draft"></a>59 [回复草稿](http://127.0.0.1:4321/kui/components/reply-draft/)<br>[reply-draft](</Users/leon/Documents/Kense ui/KUI/docs/items/reply-draft.md>) | P1 | 快捷回复/语气/长度、生成预览、编辑区与插入按钮。 | 停止/重试、生成时编辑、晚到结果确认、插入一次；流式内容和状态切换。 | 检查是否把成段替换当成流式体验；记录宿主/演示边界。 | 待重审 · `□□□□□□□□` |
| <a id="review-ask-your-data"></a>60 [数据问答](http://127.0.0.1:4321/kui/components/ask-your-data/)<br>[ask-your-data](</Users/leon/Documents/Kense ui/KUI/docs/items/ask-your-data.md>) | P1 | 问题框、SQL/结果/图表/表格布局、空/错误状态。 | 提交/停止/重试、复制真实SQL、切换结果视图；图表入场、切换和溢出。 | 核对数据映射及各视图一致，样例查询不可冒充真实执行。 | 待重审 · `□□□□□□□□` |
| <a id="review-pii-redaction"></a>61 [敏感信息脱敏](http://127.0.0.1:4321/kui/components/pii-redaction/)<br>[pii-redaction](</Users/leon/Documents/Kense ui/KUI/docs/items/pii-redaction.md>) | P1 | 选择范围、遮罩/标签、审阅与复制栏。 | 重叠范围、增删/撤销、编辑后偏移、复制前审阅；选择/拖动反馈。 | 当前是固定虚构匹配样例；要查复制实际结果与说明，不推断有真实检测。 | 待重审 · `□□□□□□□□` |
| <a id="review-dictation-button"></a>62 [听写输入](http://127.0.0.1:4321/kui/components/dictation-button/)<br>[dictation-button](</Users/leon/Documents/Kense ui/KUI/docs/items/dictation-button.md>) | P1 | 麦克风键→听取行、临时文字、状态标记和确认入口。 | 权限/停止/取消、最终文本只插一次、插入位置移动确认、过期回执；宽度与波形变化。 | 需要在实际编辑器内验收，演示无权限不等于真实适配已通过。 | 待重审 · `□□□□□□□□` |
| <a id="review-speech-player"></a>63 [朗读播放器](http://127.0.0.1:4321/kui/components/speech-player/)<br>[speech-player](</Users/leon/Documents/Kense ui/KUI/docs/items/speech-player.md>) | P1 | 播放栏、逐词高亮、进度/速度/暂停控制和换行。 | 启动/暂停/跳转/结束/失败/重试；字词时标、跨行定位、宿主音频同步。 | 必须检查跨行/滚动后的高亮，不只高亮第一句。 | 待重审 · `□□□□□□□□` |
| <a id="review-provider-status"></a>64 [服务商状态](http://127.0.0.1:4321/kui/components/provider-status/)<br>[provider-status](</Users/leon/Documents/Kense ui/KUI/docs/items/provider-status.md>) | P1 | 服务行/状态点、延迟/指标、事故详情、加载和空状态。 | 轮询/数据变化、展开/关闭、打开事故链接、失败恢复；行/详情高度与状态变色。 | 文档写明视觉与触摸待验；需核对具体实际服务状态而非固定绿点。 | 待重审 · `□□□□□□□□` |
| <a id="review-ghost-text"></a>65 [行内文字建议](http://127.0.0.1:4321/kui/components/ghost-text/)<br>[ghost-text](</Users/leon/Documents/Kense ui/KUI/docs/items/ghost-text.md>) | P1 | 真实插入点、灰色建议与正文、单/多行、光标和选择区。 | Tab/分词接受/Esc/编辑、IME、请求取消和晚到结果；流式建议、淡入/退出及位置稳定。 | 文档承认无淡入；精细转场与原生IME/触摸待验。 | 待重审 · `□□□□□□□□` |
| <a id="review-response-feedback"></a>66 [回答反馈](http://127.0.0.1:4321/kui/components/response-feedback/)<br>[response-feedback](</Users/leon/Documents/Kense ui/KUI/docs/items/response-feedback.md>) | P1 | 赞踩键、原因/补充面板、发送/成功/错误布局。 | 选择/反选、提交/重复提交、失败保留草稿、关闭/焦点；面板展开、选中/成功切换。 | 文档明确细节过渡和焦点交接仍待视觉及无障碍验证。 | 待重审 · `□□□□□□□□` |
| <a id="review-image-generation"></a>67 [图片生成](http://127.0.0.1:4321/kui/components/image-generation/)<br>[image-generation](</Users/leon/Documents/Kense ui/KUI/docs/items/image-generation.md>) | P1 | 提示/任务队列、比例控制、像素预览、结果卡/放大/下载布局。 | 开始/取消/失败/重试/放大，逐请求ID与晚回执；预览到成图、放大扫光和进度。 | 文档明确像素马赛克、upscale扫光和完整视觉比对未实现或未验证。 | 待重审 · `□□□□□□□□` |
| <a id="review-meeting-summary"></a>68 [会议纪要](http://127.0.0.1:4321/kui/components/meeting-summary/)<br>[meeting-summary](</Users/leon/Documents/Kense ui/KUI/docs/items/meeting-summary.md>) | P1 | 会议笔记/摘要/行动项、引用、收件目标与发送反馈。 | 生成/停止/重新生成、引用定位、编辑/复制/逐项发送/失败；流式文本、引用hover意图和状态过渡。 | 文档承认流式内容、引用hover意图和精细转场未实现或未验。 | 待重审 · `□□□□□□□□` |
| <a id="review-ai-workflow"></a>69 [AI 工作流](http://127.0.0.1:4321/kui/components/ai-workflow/)<br>[ai-workflow](</Users/leon/Documents/Kense ui/KUI/docs/items/ai-workflow.md>) | P1 | 节点/连接器/分支、运行状态、命令菜单和详情高度。 | 执行/停止/重试/分支选择、过期回执；连接器走向、路径交叉淡入、展开和重新定位。 | 文档承认路径交叉淡入、实测高度展开和可搜索命令菜单未实现。 | 待重审 · `□□□□□□□□` |
| <a id="review-training-run"></a>70 [训练任务](http://127.0.0.1:4321/kui/components/training-run/)<br>[training-run](</Users/leon/Documents/Kense ui/KUI/docs/items/training-run.md>) | P1 | 损失/评估曲线、指标、事件、错误卡与终点状态。 | 开始/暂停/取消二次确认/重试、恢复与宿主回执；平滑曲线、指针scrub、结果卡转场。 | 文档明确曲线直接更新，scrub/结果转场/平台与播报待验。 | 待重审 · `□□□□□□□□` |
| <a id="review-agent-handoff"></a>71 [人工转接](http://127.0.0.1:4321/kui/components/agent-handoff/)<br>[agent-handoff](</Users/leon/Documents/Kense ui/KUI/docs/items/agent-handoff.md>) | P1 | 机器人/人工角色、共享摘要、转交时间线、会话与发送状态。 | 请求/取消/接通/失败/重试、摘要编辑与草稿保留；流式内容、行进点、摘要展开。 | 文档承认流式内容、时间线移动点和完整视觉比对未完成。 | 待重审 · `□□□□□□□□` |
| <a id="review-llm-cost-calculator"></a>72 [模型费用估算](http://127.0.0.1:4321/kui/components/llm-cost-calculator/)<br>[llm-cost-calculator](</Users/leon/Documents/Kense ui/KUI/docs/items/llm-cost-calculator.md>) | P1 | 输入用量/模型/价格、排序/对比条、总价/单位和空/错态。 | 缓存/价格变化、非有限/负数/溢出、加载失败；重排弹簧、总价滚动与读数基线。 | 文档承认重排/总价滚动/加载错误集成及真实端验收待办。 | 待重审 · `□□□□□□□□` |
| <a id="review-agent-cursor"></a>73 [智能体光标](http://127.0.0.1:4321/kui/components/agent-cursor/)<br>[agent-cursor](</Users/leon/Documents/Kense ui/KUI/docs/items/agent-cursor.md>) | P1 | 光标/标签、正文块、选区与编辑前后、插入点定位。 | 开始/停止/接受/撤回、手动编辑/滚动/块移动；逐词写入、选择效果、光标测量和连续跟随。 | 文档承认逐词流式、选择效果和完整参考比对未实现或未验。 | 待重审 · `□□□□□□□□` |
| <a id="review-link-unfurl"></a>74 [链接预览](http://127.0.0.1:4321/kui/components/link-unfurl/)<br>[link-unfurl](</Users/leon/Documents/Kense ui/KUI/docs/items/link-unfurl.md>) | P1 | 文本内链接→预览卡的形变、缩略图/元信息、多链接与收起入口。 | 解析/取消/打开/发送、失败保草稿、Alt+Enter、拖回收起；加载→卡片→折叠和晚到响应。 | 文档承认inline morph、多预览、Alt+Enter、拖收起和播报未实现/未验。 | 待重审 · `□□□□□□□□` |
| <a id="review-tone-tuner"></a>75 [语气调整](http://127.0.0.1:4321/kui/components/tone-tuner/)<br>[tone-tuner](</Users/leon/Documents/Kense ui/KUI/docs/items/tone-tuner.md>) | P1 | 语气控制、原文/改写结果、变词高亮、复制/停止/重试布局。 | 快速调节/取消/旧响应、编辑冲突、复制成功失败；句子交叉淡入、替换词运动与结果宽高。 | 文档承认句子交叉淡入与完整参考比对未实现或未验。 | 待重审 · `□□□□□□□□` |

<a id="scenarios"></a>

## AI 状态与过程示意（10 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-model-training"></a>76 [模型训练示意](http://127.0.0.1:4321/kui/components/model-training/)<br>[model-training](</Users/leon/Documents/Kense ui/KUI/docs/items/model-training.md>) | P1 | 训练阶段、神经网络/节点、损失曲线、参数卡和控制面板。 | 播放/暂停/复位/调参、阶段定位；前向/反向信号、参数更新、曲线时钟和状态同步。 | 文档含未验证声明；必须核对是否为简化示意而非官网体验。 | 待重审 · `□□□□□□□□` |
| <a id="review-multi-region-failover"></a>77 [多区域故障切换](http://127.0.0.1:4321/kui/components/multi-region-failover/)<br>[multi-region-failover](</Users/leon/Documents/Kense ui/KUI/docs/items/multi-region-failover.md>) | P1 | 地图/区域位置、连接弧、状态/延迟/流量读数和故障面板。 | 故障/恢复/区域选择、请求结果；路由重分配、粒子行进、计数和时间顺序。 | 核对宿主数据与演示状态；现有细节/设备验证不完整。 | 待重审 · `□□□□□□□□` |
| <a id="review-data-flow"></a>78 [数据流示意](http://127.0.0.1:4321/kui/components/data-flow/)<br>[data-flow](</Users/leon/Documents/Kense ui/KUI/docs/items/data-flow.md>) | P1 | 源→处理→目标节点、端口/路径/分支、计数及状态色。 | 节点检查、暂停/隐藏、错误/恢复；空心到实心标记、路径运动、数量守恒。 | 文档明确详细路径动效、无障碍和设备行为待验。 | 待重审 · `□□□□□□□□` |
| <a id="review-vector-search"></a>79 [向量检索示意](http://127.0.0.1:4321/kui/components/vector-search/)<br>[vector-search](</Users/leon/Documents/Kense ui/KUI/docs/items/vector-search.md>) | P1 | 2.5D投影、点云/网格、文档卡、查询向量与结果排名。 | 阶段切换/播放暂停、筛选/选中；嵌入→索引→查询→结果的空间移动、相似度展示。 | 文档已承认2.5D几何与细节转场未复现；应列待补，不能因有四阶段按钮通过。 | 待重审 · `□□□□□□□□` |
| <a id="review-ai-spark"></a>80 [智能状态光芒](http://127.0.0.1:4321/kui/components/ai-spark/)<br>[ai-spark](</Users/leon/Documents/Kense ui/KUI/docs/items/ai-spark.md>) | P1 | 八道射线轮廓、尺寸阈值、小图点/大图粒子、Mark/Pill/Avatar。 | 所有状态/自动与受控/速度/暂停/隐藏；旋转收尾、双心跳、错误抖动、文字和色彩混合。 | 文档明确仍无同尺寸60Hz逐帧误差验收；无障碍等价未证明。 | 待重审 · `□□□□□□□□` |
| <a id="review-agent-loop"></a>81 [智能体循环](http://127.0.0.1:4321/kui/components/agent-loop/)<br>[agent-loop](</Users/leon/Documents/Kense ui/KUI/docs/items/agent-loop.md>) | P1 | 环/阶段节点、活动头、工具面板、步骤和时间线。 | 播放/暂停/定位/隐藏、阶段数据更新；头沿路径行进、停在当前位置和阶段联动。 | 文档明确详细视觉过渡待验；图示不执行实际Agent，需保持标注。 | 待重审 · `□□□□□□□□` |
| <a id="review-reasoning-tree"></a>82 [推理树](http://127.0.0.1:4321/kui/components/reasoning-tree/)<br>[reasoning-tree](</Users/leon/Documents/Kense ui/KUI/docs/items/reasoning-tree.md>) | P1 | 树层级、分支曲线、节点卡、评分/路径高亮与窄屏大纲。 | 展开/折叠/选择/定位、阶段切换；节点入场、评分增长、路径连接和连续回放。 | 文档明确连续动效与输入验证待办，不能只显示一棵静态树。 | 待重审 · `□□□□□□□□` |
| <a id="review-speculative-decoding"></a>83 [推测解码示意](http://127.0.0.1:4321/kui/components/speculative-decoding/)<br>[speculative-decoding](</Users/leon/Documents/Kense ui/KUI/docs/items/speculative-decoding.md>) | P1 | draft/verify/accepted token布局、空格、拒绝标记、统计。 | 阶段前进/暂停/重播、拒绝与回滚、边界接受率；token跨位置飞行、验证扫过、统计滚动。 | 文档承认跨位置token飞行及滚动统计仍与参考不同。 | 待重审 · `□□□□□□□□` |
| <a id="review-prompt-cache"></a>84 [提示缓存示意](http://127.0.0.1:4321/kui/components/prompt-cache/)<br>[prompt-cache](</Users/leon/Documents/Kense ui/KUI/docs/items/prompt-cache.md>) | P1 | 请求/缓存块、命中/未命中、token/费用/延迟/TTL读数。 | 自动播放/暂停、连续请求、失败/过期/清缓存；缓存飞行、命中动画、倒计时和柱形。 | 文档承认自动播放、cache-flight与倒计时未实现。 | 待重审 · `□□□□□□□□` |
| <a id="review-mixture-of-experts"></a>85 [专家混合路由](http://127.0.0.1:4321/kui/components/mixture-of-experts/)<br>[mixture-of-experts](</Users/leon/Documents/Kense ui/KUI/docs/items/mixture-of-experts.md>) | P1 | token、router、专家/连线、分数/容量/溢出与窄屏布局。 | top-k、容量不足/跳过/并列、选择/暂停/固定；包裹沿曲线移动、分数与选路时序。 | 文档明确实时播报及桌面/Web/视觉/无障碍待验；不得编造未知路由。 | 待重审 · `□□□□□□□□` |

## 每项必须留下的证据

一项可以分多次完成，但每完成一个阶段就更新该行和下面的逐项记录。只读取入口、目标行和该组件文档即可开始，不要求每次通读85项。

```markdown
### <component-id> · <日期> · <当前预览/源码版本>
- 本轮状态：S□ A□ V□ I□ M□ P□ D□ F□
- 官方参考：地址、可访问范围、日期、源码/素材哈希。
- 本地源码：组件/primitive/story路径与版本；是否有其他Agent同时编辑。
- 逐参数映射：参考参数/默认值 → Rust接口/默认值/宿主责任 → 证据。
- 同场景素材：官网和本地同数据、同尺寸截图/连续画面路径及时间标记。
- 具体差异：复现步骤 → 预期/参考依据 → 实际结果 → 影响范围。
- 已知近似/未实现/未验证：分别列出，禁止写成已通过。
- 平台与输入：逐项说明实际验证、模拟验证、未验证。
- 构建/安装/预览一致性：命令或可复用记录、版本/哈希。
- 下一步：需修改的点或无需修改的通过依据；未获实现任务时只记录。
```

若官网示例数据与KUI演示不同，先对齐测试数据再判断。若参数来自实测而非源码，标为实测并写方法。若公开源和运行页面冲突，登记版本差异，不能任选一个写为确定规范。

### activity-rings · 2026-11-05 · KUI HEAD 6c9a53e / 官网当日线上版
- 本轮状态：S✓ A✓ V✓ I□ M□ P□ D□ F□
- 官方参考：`https://uiarc.dev/components/activity-rings`（Pro，实测可访问）、`/components/activity-rings/markdown`（公开，SHA-256 `4f6f93a8ed65c1b3…`）、`https://uiarc.dev/r/catalog.json`（SHA-256 见下）、`https://uiarc.dev/llms.txt`（91336 B，`18d0c3c5316ce3a6…`）、`https://uiarc.dev/r/skills/arc/motion.md`（`99d8750bddc1545e…`）。**Pro 的 TSX 源码不可访问**，故本轮只有公开文档 + 线上实物 DOM/CSS，不能写成已读源码。线上根元素 `activity-rings_root__mXJSP`，外框实测 522×301，`viewBox 0 0 220 220`。
- 线上实测素材（headless Chrome + CDP，`/tmp/kv5/`）：`arc_rings_root.html`（`f8265b6262237263…`，整棵根 DOM）、`arc_rings.css`（`c529360c56522422…`）、`arc_rings_dom.json`（`0aeab3ea71916734…`）、`arc_ar_detail.json`（`738a1befba3757c7…`）。
- 本地源码：`registry/src/components/activity_rings.rs`（538 行，untracked `??`，mtime 2026-11-03 12:08，本轮未改）；story `apps/story/src/extra_story/charts.rs:1329`（`activity_goals`）、`:1355`（`activity_days`）。**未见其他 Agent 同时编辑这两个文件**；写入前已核对 `git status` 与 mtime。
- 逐参数映射（线上实测 → 本地 → 结论）：
  - 视口/尺寸：官方 `viewBox 0 0 220 220`、表盘 `11.5rem`(184px)、≥30rem 时 `13.5rem`(216px) → 本地 `w(px(184.)) h(px(184.))`、`scale = min(w,h)/220` → **一致**。
  - 环半径：官方 `99.5 / 79.5 / 59.5`、间距 20 → 本地 `outer_u = 106. - 20.*index`、`mid_u = outer_u-6.5` → **一致**。
  - 刻度：官方 ticksBase `56/44/32` 条 `line`，`stroke-width 2.2`、`color-mix(foreground 11%, transparent)`、`linecap round`；ticksFill 同数 `2.6`、`var(--ring)`；实测首条 `x1=204 y1=110 x2=215 y2=110`（半径 94→105，长 11） → 本地 `ticks = 4*round(TAU*mid_u/11/4)`、`outer=(outer_u-1)*scale`、`inner=(outer_u-12)*scale`、基 `2.2*scale` @ `ink.opacity(0.11*alpha)`、填充 `2.6*scale` @ `color.opacity(fill*alpha)` → **几何与色值一致**（本地未设 linecap，GPUI 默认 butt，官方 round，属 ≤0.5px 端帽差异，V 阶段单列）。
  - 第二圈：官方独立 `<circle class="…lap…" r="89/69/49" stroke-width 1.5 stroke-linecap round stroke-dasharray>`，半径 = `mid_u - 10.5`（外侧内侧各让 10.5）；本地 `lap = (outer_u-6.5)*scale` 即在 **mid 半径**上画 `1.5*scale` 细折线 → **不一致（见具体差异 1）**。
  - 中心读数：官方 `center` 常显，`centerValue` 用 `--text-2xl`(≥30rem `--text-3xl`) 等宽数字 + `centerUnit` `--text-base` + `centerLabel` `--text-xs`，实测默认文案 `0\n%\nof goals`、`inset: 26%` → 本地中心块 **仅在 `highlighted` 为 Some 时**渲染，内容为该环“数值+单位 / 环名”，无 `% of goals` 常显态 → **不一致（见具体差异 2）**。
  - 指针标记：官方每环一根 `markerArm` + `marker` 竖标（`width:3px; border-radius:2px; background: --foreground; box-shadow 0 0 0 2px --background`，`top/height` 随进度），随选中淡化到 `foreground 22%` → 本地**无**该元素 → **不一致（见具体差异 3）**。
  - 图例行：官方 `legendRow` `padding 8px 12px`（≥30rem `10px 12px`）、`gap 12px`、`grid-template-columns auto minmax(0,1fr) auto`、`border-radius --radius-control`、`[aria-pressed=true] background --surface-muted`、`[data-dimmed] opacity .5`；`swatch 3px × 30px`（≥30rem `32px`）`border-radius 2px`；`legendLabel --text-sm /500`、`legendValue --text-xs tabular-nums`、`legendPercent` `gap 6px`；达标圆点 `met` `16×16` 圆形 `color-mix(success 14%, transparent)` 底 + `success` 勾 → 本地 `face(...,52.,surface_radius,Tone::Clear).px(12).py(10)`、`swatch w(3) h(32) rounded(2)`、达标点 `size(16) rounded_full bg(success.opacity(0.14))` → 尺寸/色值一致；本地 padding 恒为 `10/12`（无窄屏 8px 档）、达标点为**实心圆无勾**、`aria_toggled` 而非 `aria-pressed` → 差异小项登记。
  - 周选择器：官方 `day` `padding 8px 0 6px`、`gap 4px`、`--text-xs`、`[aria-checked=true] color --foreground`、选中态 `dayHighlight` 用 `--surface-muted`；官方 mini 环 `30×30`、`viewBox 0 0 32 32`、三道 `miniTrack` `r=14/9.5/5 stroke-width 3` + `miniArc`（`stroke-linecap round`、`transform rotate(-90deg)`、`transition stroke-dasharray --duration-considered`），实测 dash 与当天数值比例吻合（如 `0.45 1` ≈ 90%） → 本地 `face(("activity-day",i), label, 36., …, Tone::Clear).flex_1().pt(8).pb(6)`，选中 `bg(p.tray)`，**但按天按钮内没有 mini 三道环** → **不一致（见具体差异 4）**。
  - 环数：官方“每个 metric 加一个环，3–4 个在 220 视口内可读”；本地遍历时 `if outer_u <= 20. { break; }`，即第 5 个环起直接不画（最多 4 环）→ 与官方“每项一环保留”存在边界差异（4 项以内一致）。
  - 默认选中日：官方 `defaultDay` **默认最后一天**，实测 `Sunday, September 20` 为 `aria-checked=true` → 本地 `day: 0`（默认第一天）→ **不一致，属 A 阶段等效映射缺口（见具体差异 5）**。
  - 语义：官方环 `role=img` 且 aria-label `Sunday, September 20: Deep work 72%, Writing 78%, Reading 55%`；图例 `aria-label` 形如 `Deep work, 130 of 180 min, 72 percent`；周选择器 `radiogroup` + roving tabindex → 本地 `Role::Image` + `aria_label` 为 “`label 值 unit / goal (pct%)` 以逗号连接”，图例 `aria_label` 为 “`label 值 / goal unit (pct%)`”、`aria_toggled`，周选择器 `Role::RadioGroup` + 方向键/Home/End + 各按钮 `Role::RadioButton` → **结构等效，措辞与属性名不同，登记差异**。
- 同场景素材：本地 `/tmp/kv5/deep/activity-rings_{light,dark,narrow,reduced,rtl,cond,motion,all}.png`（2026-11-05 17:03）。官网同场景**像素截图未能取到正文层**（Next.js 文章体在独立合成层，`Page.captureScreenshot` 该层恒为空白，`fromSurface` 真/假均只出侧栏，已耗时多轮仍无解）；改用**线上实物 DOM/CSS/坐标实测**作为同场景依据，其精度高于截图像素比对，已逐条列入上方映射。
- 具体差异（复现 → 预期/依据 → 实际 → 影响）：
  1. **第二圈半径**：`ActivityGoal` 目标 500/30/12，把某天值设为目标的 1.5 倍并观察越过 100% 后的续画 → 预期按官方 `<circle r=89/69/49>` 描在环的**内侧**（=mid−10.5）→ 实际本地在 **mid 半径**上画 `1.5*scale` 细折线（`activity_rings.rs:303`）→ 影响：超额幅度读错，第二圈与外环刻度重叠，正是用户所说“差异很大”的可见来源之一。
  2. **中心读数缺省态**：打开组件不悬停不点选 → 预期官方常显 `0\n%\nof goals`（并带动画/计数）→ 实际本地中心为空，仅 hover/active 才出现该环数值 → 影响：默认态信息量不足，与官网首屏结构不同。
  3. **缺少指针标记**：官方每环沿刻度有一根 3px 前景色 `marker` 指示当前进度端点并随选中淡化 → 实际本地无此元素 → 影响：弧末位置缺少指示，视觉结构少一层。
  4. **周选择器缺 mini 环**：官方每个日期按钮是 `30×30` 三道 mini 环（`r=14/9.5/5`，`sw 3`，`dasharray` 按当天三项目标比例），实测 dash `0.45 1` 等与数据吻合 → 实际本地日期按钮只有文字标签 + 选中底 → 影响：无法一眼比较各天进度，与官网“周条”外观差别明显。
  5. **默认选中日**：官方 `defaultDay` 默认**最后一天** → 实际本地 `day: 0` 默认第一天 → 影响：首屏展示的日期、迷你环与中心值均与官网不同。
  6. 次要：本地欠缺官方 `--text-2xl/3xl` 中心字号分档、图例窄屏 `8px` padding 档、达标圆点内的勾号；`aria_toggled` vs 官方 `aria-pressed`。
- 已知近似/未实现/未验证：
  - 近似（已核对通过）：环半径/间距/刻度数/刻度长/描边宽/基色透明度/选中淡化 0.22/图例间距与色条尺寸/`viewBox` 全部与线上实测一致。
  - 未实现：官方中心常显读数、`marker` 指针、日期按钮 mini 三道环、第二圈内移半径、默认选中末日。
  - 未验证：**M 阶段连续动效未达标取证**——需“同速并排录像或足够连续时间标记帧、60Hz 主要起止误差 ≤2 帧”。本轮官方正文层无法录像（同上），故 M 留 `□`。本地侧已记录风险：`ring.progress.duration(progress.clamp(0.,2.), 0.9, cx)`（`activity_rings.rs:174`）经 `registry/src/primitives/family.rs` 的 `Slide` 为**欠阻尼闭式弹簧**（`omega=5.236/0.9≈5.82`，`sample()` 有超调项），而官方 motion.md 状态值用 `spring.smooth`（0.4s，bounce 0）并“land without overshoot”，且官方环由 SVG stroke-dashoffset 驱动、**两次变更之间无动画循环**。
  - 未验证：P/I/D/F 全部待验；触摸与屏幕阅读器播报未在真机验证；`aria-pressed`/`aria-checked` 与官方措辞差异未在无障碍树比对。
- 平台与输入：本地仅以 headless Chrome 渲染 gallery（`http://127.0.0.1:8080/?story=activity-rings`）取浅/暗/窄/减少动效/RTL/条件态静态帧，**未验证**原生桌面实机、真触摸设备、键盘走查（周选择器方向键/Home/End 仅静态读代码，未实按）、系统减少动效。官方侧未在 Safari 或 320px 复测。
- 构建/安装/预览一致性：本轮只读取源码与预览，**未执行构建**；`registry/src/components/activity_rings.rs` 与 `apps/story/src/extra_story/charts.rs` 均未修改，故无新哈希。D 阶段要等有实现改动或明确指令后再跑安装/编译记录。
- 下一步：按上述 6 条差异修改 `activity_rings.rs`（优先 1、2、4、5 的结构差异，再调 3、6）；M 阶段需先解决官网同场景录像/连续帧取证手段或改用逐帧 DOM 采样（`stroke-dashoffset` / `dasharray` 时间序列）后再判。本轮无实现任务，只记录。

### sankey-flow · 2026-11-05 · KUI HEAD 6c9a53e / 官网当日线上版
- 本轮状态：S✓ A✓ V✓ I□ M□ P□ D□ F□
- 官方参考：`https://uiarc.dev/components/sankey-flow`（Pro，实测可访问）、`/components/sankey-flow/markdown`（公开，SHA-256 `632ec39d39d7438c…`）、`/r/catalog.json`、`/llms.txt`、`/r/skills/arc/motion.md`。**Pro 的 TSX 源码不可访问**，只有公开文档 + 线上实物 DOM/CSS。线上根/卡 `sankey-flow_card__1DLbJ`，实测 `<svg w=582 h=340>`、`<canvas 300×150>`。
- 线上实测素材（`/tmp/kv5/`）：`arc_sankey_dom.json`（`3ec1bfe54d1b9151…`，含 17 个 `uniqClasses`、`figText` 全文、按钮 `aria-pressed`）。
- 本地源码：`registry/src/components/sankey_flow.rs`（1210 行，untracked `??`，mtime 2026-11-03 12:07，本轮未改）；story `apps/story/src/extra_story/charts.rs`（`SankeyFlow::new("Signup journey", nodes, links).with_description("Signups by stage")` + `with_columns([Source,Activation,Plan,90 days])` + `with_periods(["Q2","Q3"],1)`；数据 `sankey_data` 9 节点 / 4 列 / 13 链接，`step 0` 全部乘以 0.85）。**未见其他 Agent 同时编辑**。
- 逐参数映射（线上实测 → 本地 → 结论）：
  - 结构：线上 `top`(标题+meta+周期按钮) → `figure` → `stages` → `chart/plot/inks`（SVG）+ `canvas`（粒子）+ `labels` + `tooltip` → `footer/flowToggle`(`Flow` 按钮) + `srOnly`；`figText` 实测为 `Signup journey\n\n11,200 signups, Jul to Sep 2026\n\nQ2\nQ3\nPlan\n90 days\nFlow` → 本地为 `title` + `hint(details)`（“悬停或点击查看流向”）+ 周期按钮行 + 列头行 + `graph`(canvas 粒子) + `hint(details)` + `Flow` 按钮 → **结构大体对应，但官方有 meta 汇总行、列头在图上方独立 `stages` 行**，本地列头由 `row().justify_between()` 平铺且**无 meta 行** → 差异见 1、2。
  - 尺寸：官方 `height` 默认 340、`<420px` 直立版至少 480 → 本地 `chart_height = if width < 420. { 480. } else { 340. }`（`:665`）、`vertical = width < 420.`（`:166` 邻域）→ **一致**。
  - 粒子：官方“一张 canvas，**每条流带上限 34**，离屏停”；本地 `let count = ((band.thickness / 8.).ceil() as usize).clamp(1, 12);`（`:842`）→ **上限 12 vs 官方 34，不一致**。
  - 流带/节点配色：官方“Sources wear the **series palette derived from the accent**，trace 用其源色”；线上实测 3 条系列的 `miniArc/lap` 色为 `oklch(0.6 0.17 275)` / `oklch(0.75 0.15 68)` / `oklch(0.66 0.09 160)`（同一套 indigo/amber/sage 系列）→ 本地未聚焦流带为 `p.ink.opacity(0.09)`（**单色灰**，`:833`）、聚焦 `color.opacity(0.36)`（`:835`）、未聚焦节点 `p.panel.blend(p.ink.opacity(0.28))` → **不一致（见具体差异 3）**。
  - 动效时长：官方“首次从中心生长”，`motion.md` 给 `spring.smooth 0.4s bounce 0`、`spring.morph 0.42s bounce 0.16`；本地首帧 `0.95s`、数据切换 `0.34s`（`:606`），提示框 `tip_x/tip_y.duration(left, 0.26)`（对应官方 `spring.snappy 0.26s`）→ **首帧与形变时长不一致，见 4**。
  - 交互：官方 Tab 聚焦并高亮首节点、列内上下/跨列左右/Home/End/Escape；本地有 `FlowPeriodChange`/`FlowInspect` 事件、周期按钮 `face(("flow-period", i), …, Tone::…, 34.)`、粒子开关 `face("flow-particles", …, 34., …, Tone::Primary)`，并实现 `connected()` 按比例 pours-through 追踪 → 功能对应，细节待 I 阶段逐键走查。
  - 语义：官方“图是一个可聚焦组，逐条流有**视觉隐藏表**列 value 与 share of source”、“标签/粒子/流带 aria-hidden”；本地有隐藏 `Role::Table`/`Role::Row` 的 `flow-data` ARIA 表（`:599` 邻域）→ **语义等效**。
- 同场景素材：本地 `/tmp/kv5/deep/sankey-flow_{light,dark,narrow,reduced,rtl,cond,motion,all}.png`（2026-11-05 17:04）。官网像素截图同样**无法取到正文层**（原因同上）；改用线上 DOM/CSS/`figText`/按钮态实测作同场景依据。
- 具体差异（复现 → 预期/依据 → 实际 → 影响）：
  1. **缺少 meta 汇总行**：打开示例 → 预期标题下有一行 `11,200 signups, Jul to Sep 2026`（官方 `sankey-flow_meta__jYgEO`，实测文案）→ 实际本地标题下直接是副标题/列头，无总量与区间汇总 → 影响：首屏信息层级与官网不同（用户所说“完全不同”之一）。
  2. **列头位置与内容**：官方列头在图中上方独立一行 `Plan / 90 days`，直立布局隐藏；本地 `row().justify_between()` 平铺于按钮区下方且宽度分散 → 影响：标签对齐与留白观感不同。
  3. **未聚焦流带用单色灰**：不悬停任何节点/流带 → 预期按官方“series palette derived from the accent”保留源色（仅降低明度/透明度）→ 实际本地为 `ink.opacity(0.09)` 纯灰 → 影响：整图默认看起来是灰带交织，正是本地实拍与官方气质差距最大处。
  4. **粒子密度与时长**：同一流带 → 官方每带至多 34 粒、首帧 `spring.smooth 0.4s`、切换 `spring.morph 0.42s`；本地每带至多 12 粒、首帧 0.95s、切换 0.34s → 影响：流动感偏稀、入场偏慢、切换偏快，节奏与官网不一致。
- 已知近似/未实现/未验证：
  - 近似（已核对）：直立布局阈值 420/高度 480、默认高度 340、粒子开关与周期按钮的 34px 高度、隐藏 ARIA 表、Escape/Home/End 键位。
  - 未实现：meta 汇总行、系列色流带、官方粒子上限 34、官方 `spring.smooth/morph` 时长。
  - 未验证：M 阶段连续动效（同上，官网正文层无法录像，本地亦未做时间标记帧）；I 阶段真实键位/触摸“点选→再点清除”、tooltip 贴边约束与离屏暂停未实测；F 阶段帧预算与离屏不空转未采样。
- 平台与输入：本地仅 headless Chrome gallery（`http://127.0.0.1:8080/?story=sankey-flow` 邻域）取静态多态帧，**未验证**桌面原生、真触摸、键盘全路径、Safari/320px。官方侧未在触摸设备复测。
- 构建/安装/预览一致性：本轮只读取源码与预览，**未执行构建**；文件未改动。
- 下一步：按 1–4 顺序改 `sankey_flow.rs`（meta 行 → 列头布局 → 系列色 → 粒子/时长）；M 阶段取证方式同上需先解决。本轮无实现任务，只记录。

### rating · 2026-11-05 · KUI HEAD 6c9a53e / 官网当日线上版
- 本轮状态：S✓ A✓ V✓ I✓ M□ P□ D✓ F□
- 官方参考：`https://uiarc.dev/r/rating.json`（20317 B，SHA-256 `83f7a2ad…669d8a`，当日线上版），含 `registry/components/rating/rating.tsx`（15846 B，SHA-256 `d55bd061…f79494`）与 `registry/components/rating/rating.module.css`（3190 B，SHA-256 `f62d971f…2f64bf9`）。依赖 `lucide-react`、`motion`。官方文档 `https://uiarc.dev/components/rating/markdown`。**Pro/Free 判定：源码 JSON 为公开可访问，未绕过任何付费或登录。**
- 本地源码：`registry/src/components/rating.rs`（43471 B，mtime 11-05 08:25）；story 演示 `apps/story/src/extra_story/selection_demos.rs`（23900 B，11-05 08:03）；读数与共享动效 `registry/src/primitives/color_motion.rs`、`registry/src/primitives/family.rs`。三者均为**未跟踪新增文件**（`??`），无他人并发编辑迹象，故本轮修改安全。
- 逐参数映射（官方 → KUI）：
  | 官方参数/默认值 | KUI 对应 | 证据 |
  | --- | --- | --- |
  | `.stars{--cell:36px;--glyph:24px;gap:2px}`（md）、`.sm --cell:28/--glyph:18`、`.lg --cell:44/--glyph:30` | `Size::Md` 36×36 命中 / 24×24 星 / 2px；Sm 28/18；Lg 44/30 | rating.md 尺寸表；`rating.rs:1110` 尺寸单测通过 |
  | `strokeWidth={1.75}` | 1.75 单位圆端点描边 | `rating.rs` 描边参数；rating.md |
  | `.field{gap:6px}`、`.row{column-gap:8px;row-gap:2px}` | 字段纵距 6px、文字↔星 8px、折行 2px | rating.rs 布局 |
  | `.base svg{fill:transparent;stroke:var(--border-strong)}`；`.stars[data-rated] .base svg{fill:var(--border);stroke:var(--border)}` | 未评分描边 `border_strong`；已评分后未亮星软化为 `colors.border` 实心，0.24s (=duration-standard) 过渡 | `rating.rs:613` `rated`、`:642` `base_fill`、`:647` `base_stroke`、`:663` `retarget(...,0.24)` |
  | 半星：第二个 radio 覆盖前半（`.half{inset-inline-end:50%}`） | 半星独立焦点、Home 选第一步、数字 1–9 直达 | `rating.rs:1110`、单测 `half_steps_have_independent_focus_and_home_selects_the_first_step` 通过 |
  | 填充 `visualDuration=0.26,bounce=0` 逐星 0.035s 错峰；提交层 0.4s；按下 0.86，未亮 0.9 | `delayed(fill_target,0.26,order*0.035)`、`:630 duration(..,0.4)`、`:635` 0.9、`:633` 0.86 | `rating.rs:629-640` |
  | 变词 0.24s `ease(0.16,1,0.3,1)` 位移 `dir*0.3em`；退出 0.12s `ease(0.22,1,0.36,1)` 反向 `dir*0.25em` | 入场 0.24s 位移 `(1-enter)*4.2*dir`（4.2px = 0.3em@14px）、退出 0.12s 位移 `-exit*3.5*dir`（3.5px = 0.25em） | `rating.rs:903`（`age/0.24`、`age/0.12`）、`:922`（`4.2*dir`）、`:933`（`-3.5*dir`） |
  | 变词模糊 `wordMotion.enter.filter=blur(motionTokens.blur.soft)`、`exit.filter=blur(motionTokens.blur.subtle)`；官方 token subtle/soft/text = 2/4/8px | 入场 `(1-enter)*4.`（soft 4px→0）、退出 `exit*2.`（subtle 2px）；GPUI 无内容模糊，用 5-tap 偏移 `±0.7px` 近似，`blur>0.22` 时启用、不透明度按 4.3 折算 | `rating.rs:945`（`taps`）、`:947`（`/4.3`）、`:951`（`±0.7` 偏移）；官方 token 取自 `https://uiarc.dev/r/skills/arc/motion.md`「blur.subtle / soft / text = 2 / 4 / 8px」 |
  | 读数预留最宽字形宽度 | `width = max over steps of shape(text).width` | `rating.rs:898-901` |
- 同场景素材：本地 `http://127.0.0.1:8080/?story=rating&lang=en`，本轮重拍 `/tmp/kv5/r3_*.png`（初始/悬停 3/离开/点 1/3/5/快速 5→1→3）；变词与面板交叉淡出本轮重拍 `/tmp/kv5/wb2_*.png`、`/tmp/kv5/panel_*.png`、`/tmp/kv5/swap_*.png`。官网正文字段无法 headless 截像素，官方依据取自 TSX/CSS/`motion.md` 数值（见上）。
- 变词入场/退出（本轮补测，19:10）：点第 5 星后对读数区（clip 4× 放大）连拍；首帧即 `Not rated → Excellent`，墨心下移 `+1.9px`（`yc 19.72→20.52`，与官方 `enter` 的 `dir*0.3em` 向下进入一致）、宽度收 `525→522`，且墨迹像素量在过渡中**双词共存**（`n 2270 → 3529 → 3603`），随后收敛到单值 3603——即旧词淡出与新区淡入同帧叠加，不是硬切或闪白。折返 `5→1` 中途改 `1→5` 时不重置到起点，读数从当前可见态接续。
- 说明面板退出交叉淡出（本轮补测，19:10）：5 星后面板出现「Thanks, Emma will see your rating / Add a note」；点「Add a note」切到「Anything Emma should know? + 输入框」。`64ms` 帧拍到旧行**已变暗**（峰值 `51`）、而 `swap_f2` 帧显示新输入框已就位，`141ms` 帧旧行复位至稳态并整体下移（`yc 44.6 → 139.8`）——证明旧内容在退出时以降不透明度、下移 6px（`exit_y = exit*6`）与新版交叉，而非瞬时替换。headless 截屏约 130ms/帧，未能逐帧量化官方 0.12s 退出曲线，属取证上限。
- 具体差异：本轮实测**未发现新的几何差异**。星心 pitch 实测 38px（`292.5 / 330.5 / 368.5 / 406.5 / 444.5`），与官方 cell 36 + gap 2 完全一致；此前"点第 3 星得到 4 星"经复测确认为**本地取样点误用了 48px 间距**，非组件缺陷——按 38px 重测后：初始 0 → 悬停 3 → 离开 0 → 点 1/3/5 分别为 1/3/5 → 快速 5→1→3 末值 3（旧 240ms 计时未污染）。
- 键盘实测（点击第 1 星取得组焦点后）：→ 逐级 1→2→3→4→5；Home→1、End→5、数字 4→4、Backspace→0；已提交 3 后 Enter 与 Space 连按**保持 3 不变**（旧"Enter/Space 清空"缺陷已修并复测）。
- 快速改分时序（本轮单测复核）：`cargo test -p kui-story --lib selection_demos` → 2 passed。`rating_follow_up_cancels_stale_delays_and_clearing_resets_the_draft` 断言先点 2 星、过 239ms 尚未出面板、再点 5 星后旧 240ms 计时被取消（`follow()==None`），过 239ms 才出现 `Follow::Thanks`，随后 Backspace 归零并清空草稿与 pending；`note_skip_send_edit_preserves_the_draft_and_focus` 断言 Not now/Add a note/Send/Enter 全路径保留草稿 "Missing keyboard states" 且在 Send 后 `edit_focus` 仍持有焦点。
- 已知近似/未实现/未验证：读数模糊为多偏移绘制近似（GPUI 0.3.10 无内容模糊），与浏览器原生 `filter: blur()` 的像素级一致性**未验证**；无障碍桥接不能完整表达 Web `aria-live`、独立 `disabled`、隐藏表单输入；半星独立焦点、受控宿主确认与尺寸表**仅单测覆盖，未逐项网页实测**。
- 平台与输入：本轮实测 Web（headless Chrome 900×760，含鼠标点击、键盘、深色）；**未验证**桌面原生、真触摸、Safari。触摸按 rating.md 旧记录（`Input.dispatchTouchEvent` 轻点第 3 星提交）保留，但本轮未重跑。
- 构建/安装/预览一致性：`cargo test -p kense-kui --lib rating` → 7 passed；`cargo check -p kense-kui`（touch 后强制重编）通过；`cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features` 与 `cargo check -p kui-web --target wasm32-unknown-unknown` 均通过；`cargo run -p kui-build -- build` 幂等（对比备份 `website/public/r` 与 `md` 各 0 差异、`llms.txt` 一致）与 `verify-preview`（235 files hashed）均 ok；空白项目 `kui init` + `kui add rating radio-cards ai-spark` 后原生与 `wasm32-unknown-unknown` 均编译通过（仅 dead_code 警告）。
- 下一步：M 阶段需解决官网同速对照取证方式（官方正文层不可录像）；若能复现官方实例，补桌面原生与真触摸；否则保持本阶段留待验。

### radio-cards · 2026-11-05 · KUI HEAD 6c9a53e / 官网当日线上版
- 本轮状态：S✓ A✓ V✓ I✓ M□ P□ D✓ F□
- 官方参考：`https://uiarc.dev/r/radio-cards.json`（13414 B，SHA-256 `69d9bf6e…34bc37`，当日线上版），含 `registry/components/radio-cards/radio-cards.tsx`（8226 B）与 `registry/components/radio-cards/radio-cards.module.css`（3877 B，SHA-256 `5f6e5cd9…36eca7a`）。依赖 `motion`。**公开可访问，未绕过付费或登录。**
- 本地源码：`registry/src/components/radio_cards.rs`（34776 B，mtime 11-05 07:33）；story `apps/story/src/extra_story/selection_demos.rs`。均为**未跟踪新增文件**，无他人并发编辑。
- 逐参数映射（官方 CSS → KUI）：
  | 官方值 | KUI 对应 | 证据 |
  | --- | --- | --- |
  | `grid{repeat(auto-fill,minmax(min(100%,180px),1fr));gap:10px}`；`list{gap:8px}` | 网格 gap 10、`auto-fill` `floor((avail+10)/(min+10))`≥1；列表 gap 8 | radio-cards.md 尺寸节；`radio_cards.rs` 布局 |
  | `.card{padding:14px 16px;border:1px solid var(--border)}`；list `padding:13px 16px 13px 14px` | 网格 `14px 16px`；列表 `上/下 13、左 14、右 16` | radio-cards.md |
  | `.ring{border:1.5px solid var(--control-on)}` | 移动轮廓 1.5px，基于实际排版边界 | `radio_cards.rs` 轮廓 |
  | `.indicator{18px;border:1.5px}`；`.indicatorDot{6px}`；icon `18px`；`gap:8px` | 圆点外框 18/描边 1.5、内点 6、图标 18、图标↔标签 8 | radio-cards.md |
  | `.root[data-layout=grid] .meta{margin-top:10px;font-size:var(--text-lg)}` | 底部信息前距 10px、字号 18px、行高 1.4，同行对齐 | radio-cards.md |
  | `transition:background-color/border-color var(--duration-fast) var(--ease-standard)` = 0.16s | 背景/边框/圆点颜色 0.16s `ease(0.22,1,0.36,1)`；圆点缩放 0.26s bounce 0；轮廓 0.42s bounce 0 | `radio_cards.rs`；radio-cards.md 帧采样 |
  | `.card[data-checked]{background:color-mix(in oklab,var(--control-on) 4%,var(--surface))}` | 选中底 = `control-on` 4% 与 surface 的 Oklab 混色 | `radio_cards.rs:425` `mix(colors.surface, colors.accent, 0.04)` |
  | `.card{border-radius:20px}`、`.ring{border-radius:20px}` | KUI `radius.md = 8px` | **登记的产品差异**（DESIGN.md 形状规则），radio-cards.md 已注明 |
- 同场景素材：本地 `http://127.0.0.1:8080/?story=radio-cards&lang=en`，本轮重拍首帧序列 `/tmp/kv5/rcx_*.png`、125% `/tmp/kv5/rc125_*.png`、`/tmp/kv5/first125.png`、LTR/RTL `/tmp/kv5/rc_ltr.png`、`/tmp/kv5/rc_rtl.png`、键盘 `/tmp/kv5/kb_ltr_*.png`、`/tmp/kv5/kb_rtl_*.png`。
- 具体差异：本轮实测**未发现新的几何差异**。官方圆角 20px 与 KUI 8px 的差异属用户已明确的矩形 8px 形状规则，非缺陷。
- 首帧无跳变（本轮补测，19:00）：900px 下页面重载后连拍 10 帧，卡片区墨迹包围盒恒为 `(80,40,819,637)`；125% 缩放（CSS 720×608、deviceScaleFactor 1.25 → 900×760 设备像素）下连拍 8 帧恒为 `(80,40,819,639)`——逐帧一致，仅选中轮廓不透明度渐入，符合"首次定位仅 0.16s 淡入、不从原点滑来"规格，**非布局跳变**。稳定帧渲染正常（grid 三列、list 四项，选中项 `Two bags` 与 `Standard`）。
- RTL 方向继承与横向键盘反转（本轮补测，18:59，运行时实测）：gallery 方向由 `hostDirection()` 读 `documentElement.dir` 决定（`apps/web/src/lib.rs:56`），方向只在 `show_story` 时写入全局 `UiDirection`。设 `dir=rtl` 后 `kuiPreview.show('radio-cards', false)` 重渲染——网格整体镜像为 `Bulk | Two bags | One bag`、第五项 `Decaf` 落到右侧起始边，列表指示圈与文字右对齐、价格移左，选中轮廓正确跟随；RTL 下按 `ArrowRight` 把选中从源序 1（Two bags）移到源序 0（One bag，显示在最右），`ArrowLeft` 再回源序 1——即**右方向键移向视觉右侧**，横向轴反转；LTR 下同一 `ArrowRight` 则从 Two bags 移到 Bulk（源序 2，视觉右移）。与单测 `horizontal_arrows_reverse_only_under_rtl`、`rtl_grid_mirrors_columns_while_keeping_source_order` 结论一致。
- 已知近似/未实现/未验证：GPUI 固定列网格需首轮测量后更新列数（已由首帧零不透明度门控掩盖）；富文本标签需宿主转原生数据；GPUI 无障碍桥接不能完整表达 HTML `required/disabled`。
- 平台与输入：本轮实测 Web（headless Chrome 900×760 与 125% 缩放，深色，含鼠标点击与 LTR/RTL 横向键）；**未验证**桌面原生、真触摸、Safari。
- 构建/安装/预览一致性：`cargo test -p kense-kui --lib radio_cards` → 6 passed；`cargo check -p kense-kui --target wasm32-unknown-unknown --no-default-features` 与 `cargo check -p kui-web --target wasm32-unknown-unknown` 均通过；`kui-build build` 幂等（0 内容差异）与 `verify-preview`（235 files hashed）ok；空白项目 `kui init` + `kui add rating radio-cards ai-spark` 后原生与 `wasm32-unknown-unknown` 均编译通过。
- 下一步：M 阶段需同速官网对照取证；补桌面原生与真触摸后推进 P/D/F。

## 给下一位 AI 的任务

> 阅读 AGENTS.md 和本文件。原108项按用户要求排除，只处理表中指定的新组件。先查该组件行、docs/items规格、registry真实源码和公开官方参考，复现样式/布局/输入/连续动效差异，填写S/A/V/I/M/P/D/F证据。只有本地能渲染或编译成功不能判定对齐。Activity rings和Sankey flow优先；其他Agent正在修改的文件先避免覆盖，记录版本再协调。当前先审查并写Markdown，用户明确要求实现后再改代码。每完成一个阶段同步该行与桌面镜像，不批量勾选通过。

桌面镜像：[KUI 组件验收进度.md](</Users/leon/Desktop/KUI 组件验收进度.md>)。仓库本文件是审查计划原件；同步时复制完整文件，避免桌面和仓库出现不同结论。旧 `scripts/update-audit-status.py` 尚未适配这轮重审表，不用它覆盖本表或桌面镜像。

## 原通过记录的处理

下面 16 项曾在旧表勾选视觉或动效通过；**这些是历史状态，本轮均待重审**。未出现在下面的新增组件同样列入上方85项，不代表它们无需审查。原108项默认通过的约定保持不变。

| 历史组件 | 旧记录日期 | 本轮处理 |
| --- | --- | --- |
| rating | 2026-11-05 11:46 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| billing-toggle | 2026-11-05 08:51 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| toggle-group | 2026-11-05 08:55 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| radio-cards | 2026-11-05 11:40 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| money-input | 2026-11-05 09:21 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| elastic-slider | 2026-11-05 13:56 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| range-histogram-slider | 2026-11-05 14:40 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| ruler-slider | 2026-11-05 14:40 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| scrub-input | 2026-11-05 09:37 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| morph-select | 2026-11-05 09:46 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| tldr-toggle | 2026-11-05 10:38 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| gauge | 2026-11-05 13:59 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| treemap | 2026-11-05 14:06 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| sankey-flow | 2026-11-05 14:07 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| activity-rings | 2026-11-05 14:08 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |
| ai-spark | 2026-11-05 09:31 | 旧视觉/动效结论撤回，按本表重新逐项核对；保留原证据。 |

本次仅重整Markdown清单和审查规则，没有修改组件、网站、安装器、构建或部署代码，也没有执行新一轮组件验收或推送。
