# KUI 新增 85 项组件重新审查清单

最后更新：2026-11-10（北京时间）。本轮状态：**0 / 85 项完成对齐验收**（treemap、streamgraph、funnel-chart 2026-11-09 按官方源码/实物 DOM 重写并以 wasm 像素取证逐项验证，完成 S/A/V/I/M 五阶段 `SAVIM□□□`——treemap 37/37（squarify+钻入退回冻结修复+键盘守卫）、streamgraph 27/27（Fritsch–Carlson+wiggle 修正+指针层命中）、funnel-chart 29/29+动效 3/3（分层堆叠语义+isolate+drops+键盘，Pro 无源码走 live DOM 取证）、race-bar-chart 原验收因用户发现换月抖动已重新打开，见 11-10 专项；slope-chart 2026-11-09 按官方开源源码重写，web 实测 19/19+动效 6/6+morph 连续性单测（spread 聚心/房间→轴坐标 bug 修复/crispEdges 轴线整数对齐），完成 S/A/V/I/M 五阶段 `SAVIM□□□`；brush-chart、metric-ticker、ridgeline 已完成 S/A/V/I/M 五阶段 `SAVIM□□□`——ridgeline 2026-11-08 按官方开源源码逐项重写(全局峰值归一/Silverman/读数光标/脊形命中/官方配色)，P/D/F 待桌面原生/真触摸/安装复验；realtime-stream、sunburst 完成 S/A/I 三阶段 `SA□I□□□□`；rating、radio-cards 已完成 S/A/V/I/D 五阶段 `SAVI□□D□`，M 与 P 仍待验）。这表示尚未通过本轮验收，并不表示没有实现。

本轮第二批核对（19:12）已把两项的六条「尚待核实」逐条落地：①rating 变词入场/退出模糊参数与官方 `motionTokens.blur` 逐项吻合并实测到中间帧；②说明面板退出内容交叉淡出实测到中间态；③radio-cards 首帧无布局跳位（900px 与 125% 逐帧包围盒一致）；④RTL 方向继承与横向键盘反转实测通过；⑤两项 WASM 构建、`kui add` 独立安装（原生 + wasm32）与网页实测通过；⑥`color_motion` 抽取后 ai-spark 强制重编与安装依赖回归通过。M（同速官网对照）与 P（桌面原生／真触摸／Safari）仍缺，故仍未打勾。

**当前按用户要求同步实现与重审。先核对可用源码、文档和实物，再修改组件。单项通过必须附三维度证据。**

2026-11-11 补充。剩余 52 项的逐项改动、可复查的截图和安装结果见 [Pro 复核的第二批返修表](</Users/leon/Documents/Kense ui/KUI/docs/audits/pro-parity-2026-11-10.md:75>)。该表区分浏览器渲染、专项操作和完整动效验收。

工单分流已复测 312px 待审核分页，代码改动审阅已复测完整源码差异与独立滚动，数据问答已复测三组样例、SQL 复制及手机失败重试，敏感信息脱敏已确认中文样例回复恢复。最终构建通过 394 个组件测试、109 个 Story 测试及双语站点检查。52 项的完整同速动效、全量 Safari、原生视觉和真机触屏仍待逐项验收。[专项返修证据](</Users/leon/Documents/Kense ui/KUI/docs/audits/pro-parity-2026-11-10.md:141>)

## 目录

- [范围与旧结论纠正](#范围与旧结论纠正)
- [官网页目覆盖补核](#官网页目覆盖补核2026-11-09)
- [228 项合并范围](#228-项合并范围)
- [本轮验收门槛](#本轮验收门槛)
- [两项优先返查](#两项优先返查)
- [用户截图专项复查](#用户截图专项复查2026-11-09)
- [官网预览与第二批截图复查](#官网预览与第二批截图复查2026-11-10)
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

- 本轮最初的本地清单为 **193 项**。其中 **108 项**按用户要求默认通过、排除本轮重审，**当时新增的 85 项进入下方重审表**。后续补接组件在覆盖表中另列。划分按历史记录中的 `user_default_pass`，不凭日期或组件分类猜测。基线 Git：`6c9a53ebfa16aa59c4ab388f18cbb944e8ea6594`。
- 历史数据位于[component-audit-status.json](</Users/leon/Documents/Kense ui/KUI/docs/audits/component-audit-status.json>)，保留原记录作为证据索引。旧轮 85 项均记录过源码、原生/WASM 和安装检查，其中 16 项记录过视觉/动效通过；**这些旧勾选不等于本轮已通过，不得自动带入新表**。
- Activity rings、Sankey flow 的旧记录只证明本地浅/暗/窄屏等状态可渲染，以及约 6 帧入场有变化；没有记录同场景官网实物对照。文档还写着逐帧对比或输入验证待办，状态却被标为通过。这不足以支持“和官网一致”的结论。
- 本轮撤回新增组件的整体对齐通过结论；原始日志保留。原生/WASM/安装的旧证据可在确认源码版本、依赖和预览版本一致后复用，但必须逐项引用，不能把“能编译”“能点击”“首尾截图相似”合并为视觉、动效通过。
- 已修过的胶囊形状、刻度/价格读数等专项问题，保留它们的修复记录；专项通过不代表整个组件已完成官网体验对齐。
- 本表的“核对/审查”是待办，不预先断言这些地方都有 bug。“文档承认”或“用户指出”的问题会单独注明。审查发现实现其实已正确，应附证据通过，不必为重审而重写。
- 本轮以此 Markdown 为当前结论。旧 JSON、旧统计脚本及旧“已通过”文案属于上一轮；不得再用旧脚本把它们覆盖回桌面的重审表。

### 官网页目覆盖补核（2026-11-09）

官网注册表（SHA-256 `333ce07696c1a98674523f8fca6f5fed001cf66fb200b5c54b8b41c0e255f301`）与组件索引（SHA-256 `de9f4b820ebe8e6d43d2791518e257826ba1d8be3fee97224ec86591ce1f107e`）的 243 个组件 ID 完全一致。另有 141 个 Blocks，归为另一目录。2026-11-10 重新下载两个索引后逐 ID 核对本批 228 项。另有单独接入的 Cover flow、Ticket tear 和 Orbit logos，当前本地预览总计 231 项。原有 108 项沿用用户验收，85 项在下方重审，另有 35 项补接并单独记录。本轮追踪范围为 **135 项**，其中 12 项待覆盖。[官网注册表](https://uiarc.dev/r/catalog.json) · [官网组件索引](https://uiarc.dev/llms.txt)

下表保留最初差出的 49 个 ID，并加入官网新增的 Cursor trail draw。本批补接 35 项。Cover flow 和 Ticket tear 单列后，官网还有 **13 个 Pro 条目待接入**。Pro 先核对公开参数及预览，源码可访问性逐项记录。

| 官网页目 | 层级 | 当前状态 |
| --- | --- | --- |
| [action-morph](https://uiarc.dev/components/action-morph) | Pro | 已接入共享 Rust 预览与安装，完整体验继续验收 · `S□□□□□D□` |
| [ai-mascot](https://uiarc.dev/components/ai-mascot) | Pro | 已接入 Rust 注册表和双语预览，独立安装双目标通过 · `S□□□□□D□` |
| [black-hole](https://uiarc.dev/components/black-hole) | Pro | 缺少注册项，待核查实现与集成 |
| [booking-pill](https://uiarc.dev/components/booking-pill) | Pro | 已接入共享 Rust 与双语预览，票据、异步提交与双路径安装已核对，完整体验继续验收 · `S□□□□□D□` |
| [branch-graph](https://uiarc.dev/components/branch-graph) | Pro | 已接入共享 Rust 与双语预览，已核对公开参数、策略切换、窄屏及双路径安装，完整体验继续验收 · `S□□□□□D□` |
| [card-to-article](https://uiarc.dev/components/card-to-article) | Pro | 已接入共享 GPUI、双语预览和双路径安装。长拖、键盘、内滚动及 320px 页眉避让已复测，完整体验继续验收 · `S□□□□□D□` |
| [charge-session](https://uiarc.dev/components/charge-session) | Pro | 已接入共享 GPUI、双语预览与双路径安装，停充、拖动、键盘、重开及屏外暂停已复核，完整体验继续验收 · `S□□□□□D□` |
| [control-center](https://uiarc.dev/components/control-center) | Pro | 已接入共享 Rust 预览与安装，完整体验继续验收 · `S□□□□□D□` |
| [cover-flow](https://uiarc.dev/components/cover-flow) | Pro | 另行接入，独立安装原生与 WASM 通过。已测桌面点侧卡、惯性抛动和模拟触摸阈值。连续动效及各平台待验 · `S□□□□□D□` |
| [cube-mascot](https://uiarc.dev/components/cube-mascot) | Pro | 缺少注册项，待核查实现与集成 |
| [cursor-trail-draw](https://uiarc.dev/components/cursor-trail-draw) | Pro | 官网新项。需核对压力笔迹、激光指针衰减、提笔吸附与连续交互。公开清单未提供源码包。 |
| [date-reel](https://uiarc.dev/components/date-reel) | Pro | 已接入共享 Rust 预览和安装，公开参数及滚轮专项已核对，完整体验继续验收 · `S□□□□□D□` |
| [dns-resolution](https://uiarc.dev/components/dns-resolution) | Pro | 已接入共享 Rust 与双语预览，公开参数、六段跳转、窄屏及双路径安装已核对，完整体验继续验收 · `S□□□□□D□` |
| [dock](https://uiarc.dev/components/dock) | Pro | 已接入共享 Rust 预览和安装，完整体验继续验收 · `S□□□□□D□` |
| [dot-grid](https://uiarc.dev/components/dot-grid) | Pro | 已接入共享 GPUI 与双语预览，公开参数、三档点距、涟漪和双路径安装已核对，完整体验继续验收 · `S□□□□□D□` |
| [ghost-mascot](https://uiarc.dev/components/ghost-mascot) | Pro | 缺少注册项，待核查实现与集成 |
| [glass-card](https://uiarc.dev/components/glass-card) | Pro | 已接入共享 Rust 预览。11-09 悬停排版修复后完成专项像素比对及新版双目标安装。完整体验待验 · `S□□□□□D□` |
| [glass-tabbar](https://uiarc.dev/components/glass-tabbar) | Pro | 已接入共享 Rust 预览与安装，完整体验继续验收 · `S□□□□□D□` |
| [gravity-well](https://uiarc.dev/components/gravity-well) | Pro | 缺少注册项，待核查实现与集成 |
| [holo-card](https://uiarc.dev/components/holo-card) | Pro | 已接入共享 Rust 预览与安装。公开参数、翻面和窄屏专项已核对，完整体验继续验收 · `S□□□□□D□` |
| [image-trail](https://uiarc.dev/components/image-trail) | Pro | 缺少注册项，待核查实现与集成 |
| [ink-fluid](https://uiarc.dev/components/ink-fluid) | Pro | 缺少注册项，待核查实现与集成 |
| [install-snippet](https://uiarc.dev/components/install-snippet) | Pro | 已接入 Rust 注册表和双语预览，独立安装双目标通过 · `S□□□□□D□` |
| [keycap-keyboard](https://uiarc.dev/components/keycap-keyboard) | Pro | 已接入共享 Rust 预览与安装，公开参数和键位、按压专项已核对，完整体验继续验收 · `S□□□□□D□` |
| [liquid-tab-bar](https://uiarc.dev/components/liquid-tab-bar) | Pro | 已接入共享 Rust 预览和安装，完整体验继续验收 · `S□□□□□D□` |
| [live-visitors](https://uiarc.dev/components/live-visitors) | Pro | 已接入共享 Rust 预览与安装。公开参数、暂停与重连专项已核对，完整体验继续验收 · `S□□□□□D□` |
| [morph-nav](https://uiarc.dev/components/morph-nav) | Pro | 已重做 Rust 导航、接入独立预览与安装，完整体验继续验收 · `S□□□□□D□` |
| [orbit-logos](https://uiarc.dev/components/orbit-logos) | Pro | 已接入共享 GPUI、双语预览与双路径安装。筛选、停靠避让、连接重试和 320px 已复测；完整体验待验 · `SA□□□□D□` |
| [orbit-menu](https://uiarc.dev/components/orbit-menu) | Pro | 已接入共享 Rust 预览与安装，完整体验继续验收 · `S□□□□□D□` |
| [page-curl](https://uiarc.dev/components/page-curl) | Pro | 缺少注册项，待核查实现与集成 |
| [payment-flow](https://uiarc.dev/components/payment-flow) | Pro | 已接入共享 GPUI、双语预览和双路径安装，已核对流程和窄屏，完整体验继续验收 · `S□□□□□D□` |
| [photo-grid](https://uiarc.dev/components/photo-grid) | Pro | 已接入共享 GPUI、双语预览和双路径安装。密度、收藏、内滚动、胶片条、鼠标手势和窄屏已复测，完整体验继续验收 · `S□□□□□D□` |
| [pixel-mascot](https://uiarc.dev/components/pixel-mascot) | Pro | 缺少注册项，待核查实现与集成 |
| [prize-wheel](https://uiarc.dev/components/prize-wheel) | Pro | 缺少注册项，待核查实现与集成 |
| [referral-card](https://uiarc.dev/components/referral-card) | Pro | 已接入共享 GPUI、双语预览与双路径安装。邮箱去重、模拟流程、名单滚动和窄屏专项已复测，完整体验继续验收 · `S□□□□□D□` |
| [sand-text](https://uiarc.dev/components/sand-text) | Pro | 缺少注册项，待核查实现与集成 |
| [share-sheet](https://uiarc.dev/components/share-sheet) | Pro | 已接入共享 GPUI、双语预览与双路径安装。复制、选择、失败重试与窄屏拖动专项已复测，完整体验继续验收 · `S□□□□□D□` |
| [skeleton-morph](https://uiarc.dev/components/skeleton-morph) | Pro | 已接入共享 GPUI、双语预览与双路径安装，已复测重载、关注、窄屏和减少动效测试，完整体验继续验收 · `S□□□□□D□` |
| [social-proof-toast](https://uiarc.dev/components/social-proof-toast) | Pro | 已接入共享 GPUI 与双语预览。叠放、暂停、关闭、320px 和双路径安装已复测，完整体验继续验收 · `S□□□□□D□` |
| [sticker-peel](https://uiarc.dev/components/sticker-peel) | Pro | 缺少注册项，待核查实现与集成 |
| [stretch-refresh](https://uiarc.dev/components/stretch-refresh) | Pro | 已接入共享 GPUI、双语预览与双路径安装，拖动、内滚动、滚轮、键盘及重试已核对，完整体验继续验收 · `S□□□□□D□` |
| [theme-switch-eclipse](https://uiarc.dev/components/theme-switch-eclipse) | Free | 独立页面与安装入口已接入，同速动效与平台待验 · `S□□□□□D□` |
| [theme-switch-rise](https://uiarc.dev/components/theme-switch-rise) | Free | 独立页面与安装入口已接入，同速动效与平台待验 · `S□□□□□D□` |
| [theme-switch-split](https://uiarc.dev/components/theme-switch-split) | Free | 独立页面与安装入口已接入，同速动效与平台待验 · `S□□□□□D□` |
| [ticket-tear](https://uiarc.dev/components/ticket-tear) | Pro | 已接入共享 GPUI、双语页面和双路径安装。短拖、长拖、空格与换票已实测。连续动效与平台专项待验 · `SA□□□□D□` |
| [time-dial](https://uiarc.dev/components/time-dial) | Pro | 已接入共享 Rust 和中英文预览，工作时段、跨日与双路径安装已核对，完整体验继续验收 · `S□□□□□D□` |
| [trial-banner](https://uiarc.dev/components/trial-banner) | Pro | 已接入共享 Rust 预览，双路径安装通过，体验验收继续 · `S□□□□□D□` |
| [typewriter-terminal](https://uiarc.dev/components/typewriter-terminal) | Pro | 已接入共享 Rust 预览和安装，公开参数及时间轴专项已核对，完整体验继续验收 · `S□□□□□D□` |
| [upgrade-prompt](https://uiarc.dev/components/upgrade-prompt) | Pro | 已接入共享 GPUI、双语预览和双路径安装。布局、模拟提交、重试、提醒及 320px 专项已核对，完整体验继续验收 · `S□□□□□D□` |
| [wallet-stack](https://uiarc.dev/components/wallet-stack) | Pro | 已接入共享 Rust 预览与安装，完整体验继续验收 · `S□□□□□D□` |

## 228 项合并范围

这批包含已接受的 108 项、重审表中的 85 项和覆盖补核表中的 35 项。其余单独补接的组件另行验证。按用户指定的顺序，新增的 120 项完成逐项验收后再合并到 `main`。编译、安装、网页路由及链接检查分别记录，视觉与连续动效使用各组件的参考证据。

- **分发 D · 新增 120 / 120。** 每项在单独的空白 crate 执行 `kui init` 和 `kui add`，交替使用打包注册表与本地源码，把对应 Markdown 的 Rust 用法放进各自项目。原生及 wasm32 的 `cargo check --workspace --keep-going` 均返回 0。智能体光标额外在两条路径分别验证。另行接入的 Cover flow 也单独通过两条路径。范围、原始构建与安装日志存于 `/private/tmp/kui-228-release/`。
- **集成构建。** `cargo fmt --check`、Clippy、371 项库测试、108 项 Story 测试及四项附加测试、WASM 库与站点客户端检查均通过。最终构建产物 `c29e8d085f8076cf0e0f`，注册表校验 267 个文件，网站校验覆盖本地 229 项双语页与 939 个目的地。
- **WebKit 路由回归。** 最终版在本机 macOS WebKit 中，每种语言分五批依次进入 120 项预览，全部绘制目标组件并保持同一个 GPUI iframe。中文切换到绘制中位数 12.5ms、最大 33ms，英文中位数 12ms、最大 39ms。样本从运行时已就绪开始，另有首次换页较慢的记录。性能数据只用于本机路由回归。
- **玻璃背景首帧专项。** 用同版本 resvg 预生成演示背景，保留原图尺寸、颜色与 12px 模糊。中英文同批次的玻璃卡片从 152ms 到 14ms，玻璃标签栏从 313ms 到 19–20ms。浏览器实测卡片展开和标签栏绘制。安装源码没有改动，WASM 体积增加约 0.4 MB。连续交互帧预算留待专项采样。
- **桑基图窄屏专项。** 按官网手机档限制每组标签宽度，修复 320px 下末组标签贴边。实测中文 Q2／Q3 和英文 390px 亮色。改动后重新验证该组件的两种安装方式。见[实拍与验证记录](#桑基图窄屏标签专项--2026-11-10)。
- **合并状态。** 本轮 120 项中分发 120 项通过，完整验收 0 项。原 108 项沿用用户确认。待办集中在同场景视觉与动效、平台输入及性能；达到逐项门槛后再提交 `main`。


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

## 官网预览与第二批截图复查（2026-11-10）

以下逐项记录截图涉及的改动和剩余差异。官网公开预览实测覆盖自适应、手机和全屏。KUI 将三档入口接到同一个 Rust／GPUI 预览实例，手机档上限 390 CSS px。品牌、文案和 8px 矩形控件圆角使用 KUI 的约定。

| 组件 | 已处理的差异 | 待完整验收 |
| --- | --- | --- |
| [数据补全列](https://uiarc.dev/components/enrich-column) | 恢复七行示例与双列表头，指令放进结果列表头。列内编辑和操作各自限宽，状态内容可撑高该行。 | 各状态逐格比对、批量和单行在中断时的动效。 |
| [多区域故障切换](https://uiarc.dev/components/multi-region-failover) | 由平面圆点改为投影节点，按节点位置重新布置标签和健康色。 | 故障和恢复的完整时序、图标、流量分配和粒子。 |
| [数据流](https://uiarc.dev/components/data-flow) | 纵向布局从输入底边出线，经处理节点的独立端口，连到输出顶边。宿主可按节点 ID 提供图标。 | 处理节点侧边读数、粒子外观与同速录制。 |
| [专家混合路由](https://uiarc.dev/components/mixture-of-experts) | 示例恢复八个专家位、分数和容量条，窄屏换成两排，圆角裁切跟随控件。 | 同场景逐项核对路由视觉、Top 1／Top 2、容量溢出和停播。 |
| [推测解码](https://uiarc.dev/components/speculative-decoding) | 验证填色使用与词块匹配的圆角，保留从左向右的验证进度。 | 草稿和目标区的排版、token 飞行、拒绝回滚与滚动统计。 |
| [图片生成](https://uiarc.dev/components/image-generation) | 首屏展示点阵舞台和建议，提示词与生成控件放在舞台下。保留四张结果和宿主票据。 | 像素马赛克、放大扫光、窄屏编辑、过期回执及同速比对。 |
| [推理树](https://uiarc.dev/components/reasoning-tree) | 预览启动首个阶段，约 670px 分界。缩进大纲为整棵树预留高度，新增肘形连线、百分比分数及最终路径高亮。 | 宽屏问题列、剪枝收线、结果动画和回放同速比对。 |
| [智能体循环](https://uiarc.dev/components/agent-loop) | 环形轨道增加方向箭头，下方分别展示检查、工具调用及累计上下文。 | 官网图标和流程布局、阶段过渡及工具卡逐帧比对。 |
| [训练任务](https://uiarc.dev/components/training-run) | 损失图增加坐标参考线与双色图例，读取最近有效验证样本，区分观察中的历史样本。 | 排队流程、图表指针检查、曲线推进和结果状态转场。 |
| [动态排名条形图](https://uiarc.dev/components/race-bar-chart) | 官网实测 472px 绘图区、116px 左侧标签区，292px 条形可用宽度。取官网 12 月值和刻度坐标可反推绘图区上限等于当期最大值。KUI 修复取整上限导致的全图突跳，刻度独立交叉淡入，月份使用固定槽位。回归测试覆盖刚跨刻度门槛时第一名的实际绘制边界。 | 文字短模糊、不同尺寸同速逐帧、反复拖动和真机帧时间。撤回旧整组动效通过标记。 |
| [生成统计](https://uiarc.dev/components/generation-stats) | 趋势图改用 56×20px 区域，最多读取 48 点。增加过程速度与最终平均速度区分、数字固定字宽、延迟分段、输入与输出词元费率和 260ms 详情展开。 | 完整状态、模型标识、缓存／推理词元、复制 JSON、官网连续对照。此项是旧 108 项中的用户点名返修，不计入 85 项分母。 |
| [图片生成显现](https://uiarc.dev/components/image-reveal) | 按官网 522px 双列舞台恢复四图、8px 间隙与 30 步示例。显示进度环，显现前缘分层柔化，提供排队／实时／完成／失败及单图重试。 | 潜变量噪声、逐级预测图、分块解码、完成光泽和复位交叉淡化。 |

动态排名条形图、生成统计和图片生成显现的公开 Markdown 均可访问，公开注册地址均返回 404。本轮几何与参数依据公开文档及可操作预览，完整实现源码未核实。

- 本地预览构建 `c5d28b1ddb1c06c94280`。自适应实测 628px，手机档 390px，外层视口 320px 时画布缩为 288px 且正文无横向溢出。实测从手机切代码、返回预览、全屏再 Escape，月份与画布地址保持。侧栏跨组件时画布地址保持，已绘制的 story 标识更新。[手机档](/private/tmp/kui-nov10-review/race-verified-mobile.png) · [320px 推理树](/private/tmp/kui-nov10-review/final-reasoning-tree-320.png)
- 动态排名图前一版构建 `172fc4073e7f50f46d4f` 到最终版未修改图表源码。拖至五月，7 张浏览器截图约每 280ms 采一次。彩色条形左缘 `612–613px`、最长条右缘 `970–971px`。原生回归测试比较跨刻度门槛前后第一名的绘制边界，结果一致。截图间隔仅用于几何复核，60Hz 下的逐帧和性能仍待验。[换月采样](/private/tmp/kui-nov10-review/race-step-0.png) · [最终现场](/private/tmp/kui-nov10-review/final-race.jpg)
- 图片生成实测点击建议后开始生成，切手机和全屏保留输入及任务，完成后显示样例结果与可重试项。推理树最终可见节点、连线、分数和答案。七行数据补全表按列对齐，实际单元格可输入，运行中当前行高亮。训练完成时验证值保留为 `0.256`。另外复看了故障切换、数据流、八专家、解码及智能体循环的最终画面。[图片生成](/private/tmp/kui-nov10-review/image-generation-fullscreen.png) · [推理树](/private/tmp/kui-nov10-review/final-reasoning-tree-fit.png) · [七行表](/private/tmp/kui-nov10-review/kui-enrich-column.png) · [训练完成](/private/tmp/kui-nov10-review/final-training-run.png)
- 生成统计完成态显示 `420ms`、`88 tok/s`、`284` 和 `$0.0077`，详情可展开。图片生成显现实测失败后仅重试第三张，其他三张保留。随后复验最终版的 8px 图片圆角。[生成统计](/private/tmp/kui-nov10-review/kui-generation-stats.png) · [单图重试](/private/tmp/kui-nov10-review/image-reveal-mobile-retry.png) · [最终四图](/private/tmp/kui-nov10-review/final-image-reveal-done.png)
- Rust 格式、Clippy、368 项组件单测、108 项 Story 单测及附带 4 项测试、WASM 编译、注册表校验、228 个双语组件和 935 个静态地址通过。12 项分别以注册表和源码独立安装，文档示例通过原生及 wasm32 编译。图片显现最终圆角补修后重跑双路径安装。此轮仍需 Safari、原生桌面实物、真机触摸及同速逐帧验收。

[Research run](https://uiarc.dev/components/blocks/research-run) 是官网 Blocks 中的完整示例，独立组件审查表仍沿用现有范围。

## 智能助手与安装片段返修（2026-11-10）

用户点名返修以下已有组件；执行泳道也单列记录。根据公开文档与浏览器实际布局、操作核对。公开预览可看，未取得 Pro 完整源码，逐参数的源码一致性不能确认。这里记录已修改的差异，未完成整项对齐验收。

| 组件 | 本次更改 | 仍需核对 |
| --- | --- | --- |
| [推测解码示意](https://uiarc.dev/components/speculative-decoding) | 两条模型轨道、对齐的输出位置、三项滚动统计、草稿长度 2–8 与对照播放。模拟和宿主提供的回放共用时间轴。 | 每次拒绝的中间帧、对照模式长文换行、触摸、减少动效。 |
| [智能体循环](https://uiarc.dev/components/agent-loop) | 228px 环形区域及三站标签、宽版上下文分栏、窄版统一工具／检查位。初始上下文为零，完成不保留运行中的活动头。 | 逐步动作、工具卡交接、尾迹和暂停／恢复的同速对照。 |
| [专家混合路由](https://uiarc.dev/components/mixture-of-experts) | 宽版横向路由、八项评分和纵列专家；窄版竖向、专家格换行。Top 1/2、参数和暂停放在同一标题区。 | 路由粒子及入场逐帧、容量异常、悬停与键盘暂停。 |
| [人工转接](https://uiarc.dev/components/agent-handoff) | 「人工接入／暂无客服」分别演示；正常流程不强制失败。状态有固定图标与徽标，错误和估计时间与摘要对齐。 | 完整转接时间线、流式消息、取消后的重试及真机滚动。 |
| [执行泳道](https://uiarc.dev/components/agent-timeline) | 44px 泳道、模型标签、统一播放头与连接器、可定位时间轴及速度控制。 | 官网状态色、指针／键盘定位、拖动后的事件及移动端同速对照。 |
| [语气调整](https://uiarc.dev/components/tone-tuner) | 收件人与主题栏、词数及修改计数、可切换词高亮。模拟改写响应语气、正式程度和详细程度。 | 改写句子切换的中间帧、真机滑动和复制失败路径。 |
| [AI 工作流](https://uiarc.dev/components/ai-workflow) | 成功、失败、运行中与空状态四例；窄版步骤副文案换行，状态和移除操作占固定位置，分支缩进。 | 分支过渡、工作流项目菜单及失败重试的同速对照。 |
| [安装片段](https://uiarc.dev/components/install-snippet) | 四个 KUI 接入场景保留 Rust 命令；选项带图标，多行代码带行号，横向滚动留在代码区。 | 代码同槽交叉淡入、密钥从左擦入及精确计时。 |
| [智能体光标](https://uiarc.dev/components/agent-cursor) | 增加协作者光标；标签靠右会翻到左侧；选区按字形边界渐进显示。 | 双光标实际路径、逐段替换淡入、审阅栏同速过渡。 |
| [训练任务](https://uiarc.dev/components/training-run) | 初始排队，有正常与显存不足两种示例；增加 190px 曲线图、等待与事件空状态。 | 全过程曲线推进、轴与轮次联合进度、暂停恢复和部署状态切换。 |
| [会议纪要](https://uiarc.dev/components/meeting-summary) | 正常、错误、无转录可切换；切换后忽略旧的模拟摘要。 | 按摘要／决定／待办依次入场、引用悬停与发送时序。 |
| [提示缓存示意](https://uiarc.dev/components/prompt-cache) | 播放、单步、重置与模拟过期；请求条带区分缓存命中与处理，显示独立缓存区与累计指标。 | 缓存段飞入、TTL 环、变更断点、滚动数字及暂停中间帧。 |

- Web 专项：推测解码从 4 切到 6 个草稿 token，开启对照后两条输出同时显示；执行泳道点击时间轴跳到 12.4s 并暂停；专家路由 Top 1 单独选中一个专家，628px 预览为横排，390px 手机档为竖排。[解码对照](/private/tmp/kui-speculative-ref/local-speculative-comparison.png) · [时间轴定位](/private/tmp/kui-speculative-ref/local-timeline-scrubbed.png) · [专家路由宽版](/private/tmp/kui-speculative-ref/local-mixture-of-experts.png) · [专家路由手机档](/private/tmp/kui-speculative-ref/local-moe-mobile.png)
- Web 专项：人工转接两种场景分别到达已接入与暂无客服，正常结果可见人工回复；工作流故障出现在创建任务，重试只重新运行失败步骤；语气调整共情改写进入完成态并更新修改计数。[人工接入](/private/tmp/kui-speculative-ref/local-handoff-connected.png) · [暂无客服](/private/tmp/kui-speculative-ref/local-handoff-unavailable.png) · [工作流失败](/private/tmp/kui-speculative-ref/local-workflow-failed.png) · [工作流重试](/private/tmp/kui-speculative-ref/local-workflow-retry.png) · [语气改写](/private/tmp/kui-speculative-ref/local-tone-rewritten.png)
- 本地按 628px 画布修正环的上方标签与活动节点重叠，完成后不绘制活动点。[智能体循环](/private/tmp/kui-speculative-ref/local-agent-loop.png)

- 第二批 Web 专项：安装片段点击「生成密钥」后占位符被样例密钥替换、步骤显示勾；智能体光标在同一文档里可见 Kense 和协作者；训练第 720 步进入显存不足，失败时对应场景仍选中；会议纪要的错误与无转录均可达，顶栏位置不随状态切换；提示缓存首请求写入、后续命中，重置和单步可操作。[生成密钥](/private/tmp/kui-speculative-ref/local-install-key-generated.png) · [双光标](/private/tmp/kui-speculative-ref/local-agent-cursor-double.png) · [显存不足](/private/tmp/kui-speculative-ref/local-training-final-oom.png) · [会议错误](/private/tmp/kui-speculative-ref/local-meeting-fixed-error.png) · [缓存命中](/private/tmp/kui-speculative-ref/local-prompt-cache-results.png)
- 本地交付构建 `2d4c30e11a1ff7c881fe`。Clippy、379 个组件单测、108 个 Story 单测及另外 4 个测试、WASM 发布构建、注册表和网站检查通过。网站校验 231 项双语文档、947 个静态目标。以上返修项用注册 JSON 与本地源码分别在空白 crate 安装，文档 Rust 示例通过原生与 wasm32 编译。浏览器截图是专项证据；各项仍需对照官网同场景完整动效及 Safari／桌面实物复验。

## Pro 组件补充复核（2026-11-10）

Arc 已移植到 KUI 的 **110 个 Pro 组件已逐项核对桌面预览**。85 项的预览有明确差异，16 项初态接近，6 项取证时的播放阶段不同，3 项画布宽度不同。另有 26 项操作或同状态补测、5 项手机预览复查。每项的差异、截图、Rust 文件与后续验证条件均已记录。连续动效、Safari、原生桌面、真机触屏及完整键盘体验仍需逐项验证。[110 项 Pro 复核清单](</Users/leon/Documents/Kense ui/KUI/docs/audits/pro-parity-2026-11-10.md>)


首批覆盖 **33 / 85 项明确差异**。第二批为剩余 **52 项**逐项返修，完成浏览器渲染检查和独立安装检查。两批各项改动及尚待完整验证的场景见 [Pro 修复进度](pro-parity-2026-11-10.md#本轮修复进度)。

## 智能体循环再次返修（2026-11-10）

- [官方 Agent loop](https://uiarc.dev/components/agent-loop) 的公开交互预览确认了三站圆环、检查与工具结果定位、上下文与累计词元、窄屏共用结果槽和速度控制。KUI 用共享 GPUI 控件实现并保留 8px 卡片圆角，默认模型使用 Kense 标志。
- 中文本地预览依次单步到「收集上下文」「执行操作」；后一状态保留 1.2K 上下文并展示工具结果。在同一预览切换到 312px 手机框后状态保留，控件和结果完整可见。[修复前](/private/tmp/kui-agent-loop-before.png) · [单步结果](/private/tmp/kui-agent-loop-step.png) · [手机布局](/private/tmp/kui-agent-loop-mobile.png) · [官方宽版](/private/tmp/kui-agent-loop-arc-desktop.png) · [官方手机布局](/private/tmp/kui-agent-loop-arc-mobile.png)
- 原生流程测试、Story 测试、Clippy、WASM 发布构建、注册表预览哈希、双语站点和 947 个静态目标检查通过。示例对照不等同于逐帧验收；原生窗口、实际触屏、Safari 和同速动效仍待核对。

## 用户截图专项复查（2026-11-09）

按用户截图的编号登记。数值步进和结构文本编辑器是原 108 项中用户明确要求返修的两个案例。其余原验收范围沿用此前划分。以下为问题级验证，组件整体仍按八个阶段验收。

| 图 | 组件 | 本地改动与专项证据 | 待继续对齐 |
| --- | --- | --- | --- |
| 1 | [形变导航菜单](http://127.0.0.1:4321/kui/components/mega-menu-morph/) | 宽版两组链接和 216px 特写卡并排，浅色特写卡用独立底色，切栏按固定列宽交叉淡入。窄版改为可滚动的折叠列表。减轻阴影，箭头贴合面板。亮暗实拍确认「最新动态」和「查看更新」完整可见。 | 官网斜向穿越菜单、快速切栏同速对照及真机触摸。 |
| 2 | [形变选择器](http://127.0.0.1:4321/kui/components/morph-select/) | 收起宽度按标签测量，箭头右侧内缩 12px。展开前后触发层左上角同为 `587,517`。列表滚动后正文 scrollY 保持 252，末项「已取消」可选并原位收起。 | 官网三种字段示例、展开动效和各状态逐帧对照。 |
| 3 | [玻璃卡片](http://127.0.0.1:4321/kui/components/glass-card/) | 悬停只更新照片视差与高光。1280 × 720 浅色截图阈值取样，鼠标在内／移出后标题像素边界同为 `616,330–732,341`，正文同为 `616,386–914,417`，统计数值同为 `617,459–864,472`。 | 官网光照、视差轨迹、反切和触摸。 |
| 4 | [数值步进](http://127.0.0.1:4321/kui/components/number-field/) | 数值与单位作为整体居中，编辑文字和滚动数字共用位置。1280px 与 320px 中文实拍已确认访客通行证的单个数字位于框中轴。 | 多位数、小数、输入法和连续加减同场景复验。 |
| 5 | [结构文本编辑器](http://127.0.0.1:4321/kui/components/rich-text-editor/) | 导出区独立滚动，切换 Markdown 与 HTML 时归顶。滚轮到达最后一句时正文 scrollY 保持 468。短样例输出上限扩至 260px，最终版默认完整显示最后一句。 | 长文触屏、键盘滚动、浮动工具条与官网中间帧。 |
| 6 | [公告栏](http://127.0.0.1:4321/kui/components/announcement-bar/) | 按公开样式恢复默认中性色、窄幅左对齐、宽幅控制列镜像占位和短行动入口。初始示例为一条可关闭公告。已在 320px 视口确认文字换行、关闭键和操作入口都完整。 | 多消息切换、倒计时和关闭动效同场景录制。 |
| 7 | [在线成员](http://127.0.0.1:4321/kui/components/presence-stack/) | 示例改为标题、人数和头像同一表头，使用项目自有头像。边框计入直径，状态标记统一置于头像层之上。1280px 与 320px 中文实拍可辨四张头像、状态标记和 `+2`。 | 多人同时加入、离开、扇出及名单键盘路径对照。 |
| 8 | [仪表盘](http://127.0.0.1:4321/kui/components/gauge/) | 9% 线宽的 270° 弧和端帽合成一个轮廓。浅色三弧实拍未见端帽二次叠色接缝。 | 极值、换档和减少动效对照。 |
| 9 | [实时数据流](http://127.0.0.1:4321/kui/components/realtime-stream/) | 绘制、刻度、命中、提示框统一绘图区。X 刻度固定锚点，轴范围按刻度分段变化，普通数据直接使用数据单位，单位与滚轮数字共用读数区。保留移出视窗的采样以填接左缘。暂停保存当前滑动位置、轴范围与异常标记进度。新增末端命中和暂停快照回归测试。最终版浅色 1280 × 720，在 325ms 内截取 18 张不同曲线画面，填充横向边缘始终为 511–972px，单位字形像素边界保持 `570,456–600,467`。 | 连续同速录像、快速暂停恢复、真触摸、帧耗时。 |

公开源码核对覆盖 [Morph select](https://uiarc.dev/r/morph-select.json)、[Number field](https://uiarc.dev/r/number-field.json)、[Rich text editor](https://uiarc.dev/r/rich-text-editor.json)、[Announcement bar](https://uiarc.dev/r/announcement-bar.json) 和 [Gauge](https://uiarc.dev/r/gauge.json)。[Mega menu morph](https://uiarc.dev/components/mega-menu-morph)、[Glass card](https://uiarc.dev/components/glass-card)、[Presence stack](https://uiarc.dev/components/presence-stack) 和 [Realtime stream](https://uiarc.dev/components/realtime-stream) 按公开文档与可操作预览测量。静止截图、局部像素比对和编译结果只记入对应专项。

- 验证版本 `801f2dd3e6ccc62447ab`。Rust 格式、Clippy、组件 285 项单测、Story 107 项单测、WASM 预览构建及网站静态检查通过。九项分别从本地注册表和本地源文件两条入口独立安装，文档示例均通过原生与 `wasm32-unknown-unknown` 编译。单组件安装产生未用到共享内部方法的常规警告。
- 现场证据保存在 `/private/tmp/kui-nov09-review/`。菜单见 `menu-final.jpg` 和 `menu-dark-settled.jpg`，选择器见 `morph-anchor-closed.jpg`、`morph-anchor-open.jpg` 和 `morph-anchor-scrolled.jpg`，编辑器见 `editor-final.jpg` 和 `editor-html-final.jpg`，实时图见 `stream-moving-0.jpg` 至 `stream-moving-17.jpg`。
- 浏览器连续截帧用于检查边界位移。截图间隔不能作为 60fps 或官网同速逐帧一致的证据。Safari、原生桌面实物及真机触摸尚待复验。
- 后续构建 `ee23a0b0adb27f98f864` 复测中文数值步进、选择器、公告、在线成员、仪表盘和编辑器。实时曲线点按暂停后，两张 940px 视口截图在图卡区域逐像素一致。此结果验证暂停冻结，连续运行按上表记录。[暂停帧 A](/private/tmp/kui-nov09-review/current-stream-pause-a.png) · [暂停帧 B](/private/tmp/kui-nov09-review/current-stream-pause-b.png)
- 形变导航菜单后续构建 `b3a4e199ca182a378f4b`。官网全屏面板实测 756 × 255px、12px 内边距、横向列间距 8px、特写卡宽 216px。KUI 按可用容器缩窄列宽并沿用 8px 圆角，示例使用 Kense 站点的本地化内容。菜单根据实际排版高度展开，窗口宽度变化后安排下一帧重新计算。实测从 940px 缩到 320px 后自动折叠，重新拉宽恢复横排，两次均在点击前看到新布局。窄版列表滚到末项时外层正文 `scrollY` 保持 461px，点按后显示已选择 `updates`。宽版以 Right 切换到资源、Escape 关闭也已实测。[官网宽版](/private/tmp/kui-nov09-review/menu-reference-products.png) · [本地浅色](/private/tmp/kui-nov09-review/menu-final-light.png) · [320px 列表](/private/tmp/kui-nov09-review/menu-final-narrow-scroll.png) · [缩窄](/private/tmp/kui-nov09-review/menu-final-narrow-resized.png) · [拉宽](/private/tmp/kui-nov09-review/menu-final-wide-resized.png)
- 最新构建通过格式、Clippy、组件 298 项单测、Story 107 项单测、WASM 构建、预览源码校验和 867 个静态地址检查。新版菜单分别从注册 JSON 和本地源码安装至空白项目，文档用法通过原生和 WASM 编译。菜单源码 SHA-256 `1ad218e0778dd66746903ecb49dab8feee09d4d9598810f10624b3236394dffa`，注册包 SHA-256 `329d2f02da233671adbb098e96237e32446b7591e217759227f04c10066e3d72`。[仪表盘切换数值](/private/tmp/kui-nov09-review/current-gauge-changed.png) · [在线成员名单](/private/tmp/kui-nov09-review/current-presence-open.png)

### 2026-11-10 补查

- 重新打开全部九项。浅色菜单末项和特写卡均完整。选择器展开保持同一左上角，实测滚到「已取消」时正文 `scrollY` 保持 381.5px。编辑器导出区最后一句可见，玻璃卡片展开后移出仍保持文字位置。320px 下复查数值居中、公告换行和在线成员状态。[菜单](/private/tmp/kui-nov09-review/verified-menu-current.png) · [选择器末项](/private/tmp/kui-nov09-review/verified-morph-scroll-final.png) · [玻璃卡片移出](/private/tmp/kui-nov09-review/verified-glass-open-out.png) · [数值步进](/private/tmp/kui-nov09-review/verified-number-field-320-current.png) · [编辑器](/private/tmp/kui-nov09-review/verified-rich-text-editor-current.png) · [在线成员](/private/tmp/kui-nov09-review/verified-presence-stack-320-current.png) · [仪表盘](/private/tmp/kui-nov09-review/verified-gauge-current.png)
- 公告移除 120px 固定上限。正文按可用宽度换行，关闭时按实际高度收起。320px 的长文案 GPUI 回归确认下方内容从自然高度平滑回到顶部。本地浏览器另验证正常公告关闭的中间和结束态。[关闭中间帧](/private/tmp/kui-nov09-review/verified-announcement-bar-closing.png) · [关闭结束帧](/private/tmp/kui-nov09-review/verified-announcement-bar-dismissed.png)
- 实时图的单位跟随数字列宽度过渡。图表焦点进入时读最新可见点，滑动中右侧刻度与该点使用同一索引。Tab、Page Up、Space、Enter 和 Escape 已实际操作。官网公告栏按公开源码和文档核对，实时图按公开文档与实物验证。两者继续保留完整动效对照待验。[公告栏源码](https://uiarc.dev/r/announcement-bar.json) · [实时数据流公开文档](https://uiarc.dev/components/realtime-stream/markdown)
- 最终预览 `5e65bdcca59bf9d07735` 复验初次进入图表焦点。绘图区第一次量到宽度或宽度变化时更新提示位置，提示框贴近最新数据点，右端刻度与提示均为 59。格式、Clippy、309 项组件和 107 项 Story 单测、原生／WASM 编译、九项各两条独立安装路径及 875 个静态地址检查通过。[最终菜单](/private/tmp/kui-nov09-review/verified-menu-final.png) · [最终实时图](/private/tmp/kui-nov09-review/verified-stream-edge-final.png)

### 最新复测与补修

- 形变导航菜单首次出现时使用当前栏位和面板完整尺寸，直接淡入。切换栏目继续过渡尺寸和位置，移动的面板按实际宽度限制在导航两侧 8px 内。浅色展开中间帧可见末行及右栏，方向键切换和 Escape 关闭已实测。[展开中间帧](/private/tmp/kui-nov09-review/recheck-final-menu-opening.png) · [切换中间帧](/private/tmp/kui-nov09-review/recheck-final-menu-switch.png) · [展开实拍](/private/tmp/kui-nov09-review/recheck-delivery-menu.png) · [官网动效](https://uiarc.dev/components/mega-menu-morph#motion)
- 形变选择器首帧按标签排版。36px 行高和 2px 间距同时用于列表与高亮；空搜索下 Home／End 跳到首末可用项，有查询时保留光标操作，Alt+Up 提交，输入法合成保护仍生效。GPUI 测试核对高亮与末行的实际边界，浏览器 End 跳到「已取消」时外部正文保持原位。[末项实拍](/private/tmp/kui-nov09-review/recheck-final-morph-keyboard-last.png) · [公开源码](https://uiarc.dev/r/morph-select.json)
- 结构文本编辑器按宿主实际宽度计算换行高度。Markdown、HTML 输出均可通过键盘滚动，长文 Home／End 有回归测试。浏览器确认中文 Markdown 的最后一句及切换后的完整 HTML 可见。[Markdown](/private/tmp/kui-nov09-review/recheck-final-editor.png) · [HTML](/private/tmp/kui-nov09-review/recheck-final-editor-html.png)
- 实时数据流在 320px 视口将状态键放在标题旁，给读数和单位独立的一行。暂停后两张实拍文件哈希完全一致。玻璃卡片悬停、移出时标题墨迹边界同为 `169,250–280,261`，底图光效仍可变化。[实时图暂停](/private/tmp/kui-nov09-review/recheck-final-stream-320-paused-b.png) · [玻璃卡片悬停](/private/tmp/kui-nov09-review/recheck-final-glass-in.png) · [玻璃卡片移出](/private/tmp/kui-nov09-review/recheck-final-glass-out.png)
- 复看访客通行证的数字中轴、窄屏公告的文案与操作、头像层级和仪表盘端帽。公告默认展示一条中性色消息，与公开预览的默认场景相同。计时轮播另按上表留待完整同场景复验。[数值](/private/tmp/kui-nov09-review/recheck-final-number-320.png) · [公告](/private/tmp/kui-nov09-review/recheck-announcement-320.png) · [在线成员](/private/tmp/kui-nov09-review/recheck-presence-320.png) · [仪表盘](/private/tmp/kui-nov09-review/recheck-gauge-settled.png) · [官网公告](https://uiarc.dev/components/announcement-bar)
- 实物证据来自 `1d9d917ddae2fb903530`、`09ebe4e9bb2ed4f950c4` 与最终构建 `f4eac65d680dd6724cb2`。最终版通过格式、Clippy、356 项组件单测、108 项 Story 单测和 4 项集成测试、WASM 编译与预览哈希校验。网站验证覆盖两种语言、225 个组件和 923 个静态地址。九项各自通过注册表与源码安装，并通过原生和 WASM 示例编译。Safari、真机触屏、原生桌面和与官网同速逐帧对照仍待验，本轮状态继续按各项证据记录。

<a id="inputs"></a>

## 输入、选择与手势控件（12 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-rating"></a>01 [评分](http://127.0.0.1:4321/kui/components/rating/)<br>[rating](</Users/leon/Documents/Kense ui/KUI/docs/items/rating.md>) | P1 | 星形轮廓、半星裁切、Sm/Md/Lg尺寸、标签与读数占位。 | 悬停恢复、跨星拖选、清除/受控确认；逐星错峰、按压回弹、词级换字和说明面板中间帧。 | 本轮已按官方 TSX/CSS 核对几何（cell 36/glyph 24/gap 2 → 星距 38，与本地实测一致）、交互与键盘全路径、变词模糊（`blur.soft 4px`/`blur.subtle 2px` 与官方 token 逐项吻合，实测到双词共存中间帧）、说明面板退出交叉淡出（拍到旧行降透明度中间态）、快速改分取消旧计时（单测）、独立安装与文档同步，见下方记录；M（同速官网对照）与 P（桌面原生/真触摸/Safari）仍待验。 | 部分通过 · `SAVI□□D□` |
| <a id="review-billing-toggle"></a>02 [计费周期](http://127.0.0.1:4321/kui/components/billing-toggle/)<br>[billing-toggle](</Users/leon/Documents/Kense ui/KUI/docs/items/billing-toggle.md>) | P1 | 胶囊轨道/高亮/优惠标记、间隙与中英文宽度。 | 切换月份/年度、重复选择、方向键；高亮中途反向、阴影与文字同步。 | 已做胶囊修复；需用本轮证据核对完整体验。 | 待重审 · `□□□□□□D□` |
| <a id="review-toggle-group"></a>03 [切换组](http://127.0.0.1:4321/kui/components/toggle-group/)<br>[toggle-group](</Users/leon/Documents/Kense ui/KUI/docs/items/toggle-group.md>) | P2 | 单选/多选组尺寸、首尾圆角、分隔线、选中和悬停重合。 | 受控值、禁用跳过、方向键；单选高亮滑移与多选独立填充，快速连点。 | 旧有本地测试记录；未作为本轮官网一致性结论。 | 待重审 · `□□□□□□D□` |
| <a id="review-radio-cards"></a>04 [选项卡片](http://127.0.0.1:4321/kui/components/radio-cards/)<br>[radio-cards](</Users/leon/Documents/Kense ui/KUI/docs/items/radio-cards.md>) | P1 | 各卡片布局、选中标记、图标/标题/说明基线、描边。 | 单选、受控拒绝、禁用、键盘与触摸；标记切换和卡片反馈。 | 本轮已按官方 TSX/CSS 逐参数核对（grid/list 布局、padding 14/16 与 13/16/13/14、border 1px、ring 1.5px、indicator 18/dot 6、icon 18、meta 18、0.16s 均吻合）、首帧无跳变（900px 与 125% 墨迹包围盒逐帧一致）、RTL 网格镜像与横向键反转运行时实测通过、独立安装（原生+wasm32）与文档同步，见下方记录；官方圆角 20px 与 KUI 8px 属登记的产品差异。M（同速官网对照）与 P（桌面原生/真触摸/Safari）仍待验。 | 部分通过 · `SAVI□□D□` |
| <a id="review-money-input"></a>05 [金额输入](http://127.0.0.1:4321/kui/components/money-input/)<br>[money-input](</Users/leon/Documents/Kense ui/KUI/docs/items/money-input.md>) | P1 | 币种/金额/快捷增额布局、字宽、小数点和光标位置。 | 输入提交/失焦、非法草稿回退、上下界拒绝、快捷步进；边界拉伸和提示闪动。 | 已有步进拒绝专项修复；仍需完整源码及实物对照。 | 待重审 · `□□□□□□D□` |
| <a id="review-elastic-slider"></a>06 [弹性滑块](http://127.0.0.1:4321/kui/components/elastic-slider/)<br>[elastic-slider](</Users/leon/Documents/Kense ui/KUI/docs/items/elastic-slider.md>) | P1 | 轨道/滑块/气泡尺寸、数值和边界位置。 | 拖动/键盘/触控、过界拉伸、回弹和速度继承；滚动意图、快速反向。 | 旧入场/拖动记录不替代与官网同速轨迹对照。 | 待重审 · `□□□□□□D□` |
| <a id="review-range-histogram-slider"></a>07 [区间分布滑块](http://127.0.0.1:4321/kui/components/range-histogram-slider/)<br>[range-histogram-slider](</Users/leon/Documents/Kense ui/KUI/docs/items/range-histogram-slider.md>) | P1 | 柱形分桶、两端滑块、边界高亮、右对齐等宽区间读数。 | 双端不交叉、柱形点击、分桶吸附、计数；拖动/松手/快速往返及位数变化。 | 读数抖动已专项修复并有回归；全组件仍须重审。 | 待重审 · `□□□□□□D□` |
| <a id="review-ruler-slider"></a>08 [刻度滑块](http://127.0.0.1:4321/kui/components/ruler-slider/)<br>[ruler-slider](</Users/leon/Documents/Kense ui/KUI/docs/items/ruler-slider.md>) | P1 | 刻度间距、固定指针、边缘淡化、读数/单位基线。 | 半步固定小数、99.5→100.0进位、拖甩惯性、横向滚轮、反向与到边；数字列持续性。 | 读数跳位已专项修复；还需全轨道/惯性和官网连续帧对照。 | 待重审 · `□□□□□□D□` |
| <a id="review-signature-pad"></a>09 [签名板](http://127.0.0.1:4321/kui/components/signature-pad/)<br>[signature-pad](</Users/leon/Documents/Kense ui/KUI/docs/items/signature-pad.md>) | P1 | 画布比例、笔触端头/粗细、工具栏、空/已有签名状态。 | 鼠标/触摸/压力、撤销重做/清空/回放/导出、触摸取消；笔迹延迟和回放顺序。 | 确认导出的实际内容和宿主保存边界，不能只点击按钮。 | 待重审 · `□□□□□□D□` |
| <a id="review-scrub-input"></a>10 [拖动数值输入](http://127.0.0.1:4321/kui/components/scrub-input/)<br>[scrub-input](</Users/leon/Documents/Kense ui/KUI/docs/items/scrub-input.md>) | P1 | 数值/单位/拖动标签、编辑与非编辑态宽度、光标。 | Shift/Alt步进、表达式/单位切换、边界/环绕、拖动取消；数值稳定与弹性反馈。 | 需审当前用户正在看的实际示例与连续操作。 | 待重审 · `□□□□□□D□` |
| <a id="review-morph-select"></a>11 [形变选择器](http://127.0.0.1:4321/kui/components/morph-select/)<br>[morph-select](</Users/leon/Documents/Kense ui/KUI/docs/items/morph-select.md>) | P1 | 收起键→搜索面板的同一表面、分组行、图标、选中高亮。 | 搜索/禁用/空结果/Page键、选择/取消/焦点返回；宽高、换词、快速反开合与防裁切。 | 既有功能记录不足以证明形变中间过程一致。 | [截图返修与独立安装已验证](#用户截图专项复查2026-11-09) · 待完整对齐验收 · `□□□□□□D□` |
| <a id="review-glass-segmented"></a>12 [玻璃分段选择](http://127.0.0.1:4321/kui/components/glass-segmented/)<br>[glass-segmented](</Users/leon/Documents/Kense ui/KUI/docs/items/glass-segmented.md>) | P1 | 胶囊外轨道/玻璃底片、描边、阴影、间隙和标签。 | 拖拽/抛掷/方向键/禁用；前后沿独立弹簧、拉伸、回弹和中断。 | 胶囊已恢复；玻璃层次及不同输入路径仍需完整比对。 | 待重审 · `□□□□□□D□` |

<a id="surfaces"></a>

## 导航、消息、表格、浮层与进度（12 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-countdown"></a>13 [倒计时](http://127.0.0.1:4321/kui/components/countdown/)<br>[countdown](</Users/leon/Documents/Kense ui/KUI/docs/items/countdown.md>) | P1 | 各数字列、分隔符、单位、完成文案与宽度。 | 暂停/重置、59→00借位、到零只触发一次；最后一秒到完成态及减少动效。 | 文档明确完成过渡未采样，结束态为硬切；必须单列待补。 | 待重审 · `□□□□□□D□` |
| <a id="review-announcement-bar"></a>14 [公告栏](http://127.0.0.1:4321/kui/components/announcement-bar/)<br>[announcement-bar](</Users/leon/Documents/Kense ui/KUI/docs/items/announcement-bar.md>) | P2 | 公告高度、图标/操作/关闭键位置、窄屏换行。 | 6秒轮播、暂停与恢复、hover/focus暂停、关闭与重开；消息交叉淡入和高度收起。 | 需覆盖多个公告及关闭后的布局，不以首帧为准。 | [截图返修与独立安装已验证](#用户截图专项复查2026-11-09) · 待完整对齐验收 · `□□□□□□D□` |
| <a id="review-cookie-consent"></a>15 [Cookie 同意设置](http://127.0.0.1:4321/kui/components/cookie-consent/)<br>[cookie-consent](</Users/leon/Documents/Kense ui/KUI/docs/items/cookie-consent.md>) | P1 | 通知/偏好设置/保存后入口的尺寸、分组、胶囊开关与按钮。 | 接受/拒绝/自定义、必选禁用、取消保留策略、持久化宿主；表面展开回收及焦点。 | 复用Switch的胶囊修复有效；其余流程和动效待对照。 | 待重审 · `□□□□□□D□` |
| <a id="review-undo-toast"></a>16 [撤销提示](http://127.0.0.1:4321/kui/components/undo-toast/)<br>[undo-toast](</Users/leon/Documents/Kense ui/KUI/docs/items/undo-toast.md>) | P1 | 被移除行、通知、倒计时与撤销/关闭键位置。 | 合并批次、超时提交、hover/focus暂停、撤销后数据/焦点恢复；行收展与成功反馈。 | 要检查超时和多次删除，不只演示一次撤销。 | 待重审 · `□□□□□□D□` |
| <a id="review-tree-table"></a>17 [树形表格](http://127.0.0.1:4321/kui/components/tree-table/)<br>[tree-table](</Users/leon/Documents/Kense ui/KUI/docs/items/tree-table.md>) | P1 | 列宽、树缩进、连接线/展开键、选中/半选框和加载行。 | 展开全选、层级键盘、懒加载失败重试、排序与稳定ID；行高展开和滚动定位。 | 核对错误行是否能真实恢复，避免仅增加提示文字。 | 待重审 · `□□□□□□D□` |
| <a id="review-mega-menu-morph"></a>18 [形变导航菜单](http://127.0.0.1:4321/kui/components/mega-menu-morph/)<br>[mega-menu-morph](</Users/leon/Documents/Kense ui/KUI/docs/items/mega-menu-morph.md>) | P1 | 导航锚点、单一浮层的宽高、面板内布局与窄屏折叠。 | hover意图/跨栏目/Esc/焦点；共享面板移动、交叉淡入、快速反向和展开边界。 | 必须查面板移动途中，不能只比两个静态菜单。 | [截图返修与独立安装已验证](#用户截图专项复查2026-11-09) · 待完整对齐验收 · `□□□□□□D□` |
| <a id="review-glass-menu"></a>19 [玻璃操作菜单](http://127.0.0.1:4321/kui/components/glass-menu/)<br>[glass-menu](</Users/leon/Documents/Kense ui/KUI/docs/items/glass-menu.md>) | P1 | 触发器到菜单的形变、玻璃层/行高/图标/快捷键与子菜单。 | 子层前进返回、禁用/勾选/Esc/焦点；层间水平切换、行错峰及快速开关。 | 核对源码参数与可实现的玻璃效果差异，不以透明底替代。 | 待重审 · `□□□□□□D□` |
| <a id="review-presence-stack"></a>20 [在线成员](http://127.0.0.1:4321/kui/components/presence-stack/)<br>[presence-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/presence-stack.md>) | P1 | 头像叠放/状态点/溢出计数、展开名单、名字提示的对齐。 | hover/focus扇出、键盘点名、选人/关闭；稳定ID进出、头像位置、文字宽度与输入点。 | 旧全仓测试曾有成员选择器不匹配记录；需用当前版本重新验证。 | [截图返修与独立安装已验证](#用户截图专项复查2026-11-09) · 待完整对齐验收 · `□□□□□□D□` |
| <a id="review-toast-stack"></a>21 [通知堆叠](http://127.0.0.1:4321/kui/components/toast-stack/)<br>[toast-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/toast-stack.md>) | P1 | 前后卡露出、深度/阴影/位置、正文/操作/关闭。 | 连续推入/更新/删除、hover/focus暂停与展开、超时；布局弹簧、堆叠前移和焦点。 | 覆盖至少4条与不同高度，检查裁切及退出后正文滚动。 | 待重审 · `□□□□□□D□` |
| <a id="review-bottom-sheet"></a>22 [底部面板](http://127.0.0.1:4321/kui/components/bottom-sheet/)<br>[bottom-sheet](</Users/leon/Documents/Kense ui/KUI/docs/items/bottom-sheet.md>) | P1 | 遮罩/面板/手柄、安全区、内容区与分档高度。 | 拖动/甩动/键盘、内外滚动冲突、边界取消、关闭焦点；阻尼/投射/吸附和减少动效。 | 真实触摸与Safari需单独实测，模拟截图不算真机完成。 | 待重审 · `□□□□□□D□` |
| <a id="review-sheet-stack"></a>23 [多层面板](http://127.0.0.1:4321/kui/components/sheet-stack/)<br>[sheet-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/sheet-stack.md>) | P1 | 前后层尺寸/退距、遮罩叠加、标题栏和窄屏边界。 | push/pop/Esc/拖回、焦点层级、取消与反开；背景层深度、顶层跟手和连续返回。 | 核对只关闭顶层、滚动/命中不穿透，非两张卡静态重叠。 | 待重审 · `□□□□□□D□` |
| <a id="review-morph-loader"></a>24 [形变加载](http://127.0.0.1:4321/kui/components/morph-loader/)<br>[morph-loader](</Users/leon/Documents/Kense ui/KUI/docs/items/morph-loader.md>) | P1 | dots/bars/ring/square与成功/失败的几何、笔画、固定足迹。 | 快速换形/重复结果、暂停/隐藏；四笔独立轨迹、速度接续、收拢/回弹和最终静止。 | 已有较详细参数与局部修复；仍需所有形状同速官方对照，不能只验一个加载态。 | 待重审 · `□□□□□□D□` |

<a id="charts"></a>

## 图表、指标与活动（17 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-gauge"></a>25 [仪表盘](http://127.0.0.1:4321/kui/components/gauge/)<br>[gauge](</Users/leon/Documents/Kense ui/KUI/docs/items/gauge.md>) | P1 | 起止角/弧宽/端帽/轨道、中心数字、刻度和配色。 | 空/极值/越界/非有限值；弧与读数同步、快速更新、减少动效。 | 旧通过只显示弧和数字可渲染；需官网同场景比较。 | [截图返修与独立安装已验证](#用户截图专项复查2026-11-09) · 待完整对齐验收 · `□□□□□□D□` |
| <a id="review-animated-counter"></a>26 [动态数字](http://127.0.0.1:4321/kui/components/animated-counter/)<br>[animated-counter](</Users/leon/Documents/Kense ui/KUI/docs/items/animated-counter.md>) | P1 | 等宽数字、分组符/正负号/小数、前后缀和基线。 | 进位/借位/跨零、长短数值、快速反切、格式更新；旧列退出/新列展开和静止帧。 | 复用读数不能在动画结束时换排版；需像素与几何证据。 | 待重审 · `□□□□□□D□` |
| <a id="review-waffle-chart"></a>27 [华夫图](http://127.0.0.1:4321/kui/components/waffle-chart/)<br>[waffle-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/waffle-chart.md>) | P2 | 单元数量/间距/圆角、类别分配、图例、读数与配色。 | 图例筛选至少保留一类、键盘定位；单元稳定ID重排、交叉顺序和入场错峰。 | 核对百分比到格数的舍入与最终总数。 | **2026-11-09 对照官方 waffle-chart 实物 DOM/CSS/交互/动效实测全量重写**(Pro 无源码;取证 /tmp/kv9:wf_full.html+wf_css.css+wf.md+wf_card.json)。本轮修改：①API WaffleCategory{id,label,value,color?}(显式色优先)+label/unit/set_categories/set_label/set_visible+WaffleInspect 事件,id reconcile 复用飞行;②色序按类别数组序:i0=series1 oklch(.73 .15 250)=rgb(85,173,255)/i1 rgb(156,55,190)/i2 rgb(230,110,0)/i3 rgb(0,168,97)/i4=neutral oklab_mix(surface,ink,.40)/i5=.20/i6+=.10——oklab 全矩阵抽到公共模块 color_math.rs(srgb↔linear↔oklab,白点守恒验证,split_bar 共用);③**格数分配=累计 round**:逐类累计 share,边界=round(累计×100),每类 cells=边界差(四国反推并全部验证:de 43/4/1/15/26/11、fr 14/12/65/6/1/2、uk 34/2/14/35/1/14、us 14/6/19/43/16/2,总和恒 100);④几何 232×232 网格 100 cells、cell 20.5/pitch 23.5(gap3)/圆角 22%,**从左下角列内向上填充、列满右移**;⑤tooltip min150/max240 p8/12 r14 panel+border+shadow_lg、行=swatch10×10 r3+name 14 secondary+share 14/500 ink、meta ml18 12 faint tabular 千分位『{value} {unit}』、**定位 x=clamp(cell中心−w/2, 0, 232−w),上方 cell顶−10−h,出网格顶翻下方 cell底+10**;⑥legend face 36 r12 p10/4 gap10、swatch 10 r3、share=AnimatedCounter 16/500 tabular 右对齐 min-w51 滚轮、on 时 bg ink5%+字 ink、其余 dim .5、Enter/Space pin、aria_toggled;⑦键盘 Arrow 逐格/PageUp/PageDown 跳类别首格/Home/End/Esc,grid role Group tabindex0+官方 aria 句式;⑧动效:入场 pop 0.4+0.6α delayed slot/100×0.32+0.24 左下级联、换数据飞行 0.48s+同级联 delay+**中段 dip 谷 0.62**(官方帧实测 1→0.9997→0.935→0.676→0.620→0.641→0.816,实现 1−0.38·sin(πt))+变色 0.48s delayed、legend share 滚轮、reduced 立即;⑨demo 四国六类数据(总量=公布值 507/494/285/4178 TWh)+Country segmented+卡头 title 16/500+meta『{country}, {total} TWh』千分位;⑩卡 min(100%,640) p24 gap24 **bg surface+border+radius-surface**(本轮补齐)。**两个真实缺陷修复**:①键盘光标被 wasm key 事件触发的 hover-out 清掉→tooltip/环消失(新增 key_cursor 字段:键盘置位后 hover-out 不清,鼠标 move 复位,对齐官方 data-cursor 键盘态);②demo 卡缺 bg/border/圆角(整页透出 stage)。**平台差异登记**:roledescription/live region 官方有而 GPUI 缺或部分缺。**web 实测 17/17 判据=静态 7/7+交互 6/6+动效 4/4**(wf_verify/wf_interact/wf_motion.py,1200×1000 SwiftShader):静态含 232×232 方格/六类 slot 中心色序逐项吻合(wind 85,173,255…other N20)/cell 20-21px+gap3=20.5/23.5/煤 N40 wash/卡头标题+meta/legend 6 行 swatch/share 大字;交互含 hover 底格 tooltip 上方 67 行+文本 240px+swatch/顶行翻下方 76 行/legend pin 他类 dim 0.26(2417px)+行亮 3318px/ArrowRight 移标+PageDown 跳类(蓝 dim)/Esc 恢复/France 切换蓝 16674→5374px(43→14 格)橙 368→25216px(1→65 格);动效含入场 blues 0→7087→15824→16674 单调+col0 领先 col4 整 4 帧错峰/换国飞行橙 106→976→25721(中段≈60% 面= dip)/卡面全程 panel 无闪/legend 滚轮 ink 137→56→57→130(滚动中骤降后换终值)。fmt/clippy 4 包/232+106 测试(含累计 round 四国断言+thousands)/wasm check 双目标/build-web 绿;中文字体子集已重建(『煤』字新文案)。**取证教训**:验证脚本复用 sp_verify.load_story 实际加载的是 split-bar,导致『入场动画冻结 α≈0.67』误诊——实测页面 rAF 60fps 正常、组件无冻结,教训是跨组件复用脚本先核对加载目标;另:蓝块定位须排除图例 swatch(密度列过滤)、卡头文字抗锯齿会撞 N40/N20 容差(行密度过滤)、分段控件标签字母间隙须 6px 容差成簇、CDP 截图偶发拿到合成器陈旧帧(定态判据复拍一次)。**待验**:桌面原生、Safari/125%/RTL、真触摸、kui add、官方实物像素对照、roledescription/live region 平台补齐。 11-09 补齐 `animated-counter` 与颜色助手的安装依赖，并修正文档构造参数。独立安装及示例原生/WASM 编译通过。 | 部分通过 · `SAVIM□D□` |
| <a id="review-slope-chart"></a>28 [斜率图](http://127.0.0.1:4321/kui/components/slope-chart/)<br>[slope-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/slope-chart.md>) | P2 | 左右端点、线宽、标签避让/基线、刻度和图例。 | 悬停/选择/键盘、空值/新旧系列；端点与路径插值、淡化和提示跟随。 | 检查交叉线和拥挤标签，不仅看稀疏样例。 | **2026-11-09 对照官方开源源码逐项重写**(取证 /tmp/kv7/slope_tsx.txt+slope_css_extracted.css+slope.md 全 API/motion/键盘/a11y)。本轮修改：①API SlopeSeries{id,label,before,after}+start/end_label+value_prefix/suffix+highlight+ranks+row_height+empty_label+set_series(id reconcile)+SlopeInspect 事件；②几何 HEAD28/PAD14/LABEL_GAP20/NEAR22、leftRoom168(narrow132/116)、rightRoom ranks?96:64(narrow76/52)、plot h=max(200,n*44)、hi/lo 全值域+退化域守护、**spread_labels 官方分组聚心**(排序→合并 prev.top+len*gap>group.top→组 top=均值→clamp→迭代至稳)；③线 2px 入场 pathLength 0.7s delay 0.1+0.05i(in_out)、dot r4+2px surface 环 start 0.05i/end 0.7+0.05i scale 0.2s、标签入场 0.3/0.7+0.05i 两段；④标签 14/12px gap8/6、name secondary(accent/on→ink)、value ink(accent MEDIUM 不移位)、rank 前缀 "up N to rank R" 式读文+↑/↓/=(U+2212 MINUS)+数字滚入 roll 0.26s(方向感知 from_below)、**axis 1px crispEdges=整数对齐 fill quad**(path 1px 在 wasm 亚像素混成 34 灰已修)；⑤tooltip min168/max240 p10/12 r14 tray+title/value/tipMeta、spring 0.26s 滑到线中点、above=mid-104-16≥0、淡出 0.16s 保留内容；⑥dim 线 .14/标签 .3(focus 时)、hover 守卫 last_input_was_keyboard；⑦键盘 byEnd 序 Down/Right+1、Up/Left(at<0→0)、Home/End、Esc 清除+focus-visible→byEnd[0]；⑧nearest 含标签列命中(x<x0 用 l0/x>x1 用 l1)d≤22、touch 按住持续读/抬清；⑨grouped1 maximumFractionDigits1(4.0→"4");⑩a11y live region(名/两值/变化/名次)+summary+sr 表+stage Group role+Figure。**关键 bug 修复**:paint 闭包把 gutters 的房间对 (168,96) 误当轴坐标解构导致线反向画至 x=96(提取 axes() 共享函数+单测锚定);**调试方法论沉淀**:wasm eprintln 不进 console、用 title 探针+paint 指示条取证;抗锯齿细斜线丢纯色像素不能作线长判据;1px path 亚像素混色需整数对齐 quad。**web 实测 19/19+动效 6/6+morph 单测**(sl_verify.py/sl_motion.py,1200×713 SwiftShader):accent 562px 画满端到端/轴线 406 间距 col432/838 各 184px/卡内容 670 宽/4 行 start dots 287/302/349/365/span78=官方值域映射/Q1/Q2 居中轴上±8/左右列 name+value+rank/dot core8/9+ring/hover tooltip 167×99 tray 居中避线/neutral 1557→21 dim+accent 578 保持/ArrowDown byEnd[0]+accent 578/Esc 清;动效:入场 tip 54→126→397→461 单调无回跳/settle 429..842/glide centre y 326→318 无瞬跳/dim neutral 1341→30+白线 0→418/Esc 后消失;morph 单测:切换首采样=旧值 3.1/4.6(起止连续)+单调+终值 3.6/5.2。fmt/clippy 4 包/221+106 测试(新增 axes/spread/span/画布 670 宽/morph 连续 5 测)/wasm check/build-web 绿;中文字体子集已重建。**待验**:桌面原生、Safari/125%/RTL、真触摸、kui add、官方实物像素对照。| 部分通过 · `SAVIM□D□` |
| <a id="review-radar-chart"></a>29 [雷达图](http://127.0.0.1:4321/kui/components/radar-chart/)<br>[radar-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/radar-chart.md>) | P2 | 网格/轴数/标签方向、填充透明度、点和图例。 | 不同系列/极值/缺轴、选中/禁用；顶点/轮廓连续变形和提示位置。 | **2026-11-09 对照官方 radar-chart 实物 DOM/CSS/动效实测全量重写**(Pro 无源码;取证 /tmp/kv7/radar_dom.html+radar_pagecss.txt+radar.md,oklch 换算+官方站 morph/展开帧实测)。本轮修改：①API RadarAxis{id,label}+RadarProfile{id,label,values 0-100,color}+meta+header_extra(卡头右侧任意视图,官方 Compare with segmented)+set_profiles(id reconcile)+RadarInspect 事件;②色 series1=oklch(.73 .15 250)=rgb(85,173,255)(KUI ACCENT)/series2 rgb(156,55,190)/series3 rgb(230,110,0),差值用 U+2212 MINUS;③卡 min(100%,600px) gap24 p24(<640px 20/16) radius-surface;④svg size=min(stage_w,448) 居中、maxR=size/2−64、4 环六边形 border 1px、spoke(data-on→foreground45%)、shape stroke 2px round join+fill slot0 .12 其余 .06(选中 .2/dim .22);⑤marker 9px(fill 系列色+surface 2px 环,panel 5.5 底+color 4.5 面)滑移到 active 轴顶点;⑥axisLabel 12px secondary 三向 anchor(center96/start/end72)绝对定位 spoke 端外法向,active 非 on→55%、on→foreground;⑦tooltip min164 max240 p10/12 r14 tray+shadow 近似+行按**该轴值降序**+tipKey 10×2.5+gap 列 2.6em 右对齐(首行空)+**定位公式**:标签盒 anchor→[x−48,96]/[x,72]/[x−72,72],tx=lx+lw+12 右优先/放不下左转/clamp(8,stage−w−8),ty=(P.y−12−h/2).clamp——顶轴实测 (335,9.5) 精确复现(屏幕 (664,168) vs 官方 (661,166.5));⑧legend pill 32 高 keyLine 14×2.5(官方胶囊 br9999 例外)+pressed bg ink5%+dim 50%;⑨键盘 ArrowRight/Down+1 顺时针/Home/End/Esc+role Group/tabindex/roledescription+aria-label;⑩motion:换 profile 弹簧 morph 0.45s(官方实测也 ~1 帧,KUI 观感对齐)、首视 web 展开 reveal 0.9s 半径展开、marker/tooltip 滑移 0.16s、reduced 立即;⑪demo 官方 6 轴 Lisbon/Berlin/Toronto/Austin 数据+Compare with segmented。**关键 bug 修复**:Slide::duration(v, 0.)→omega=∞→sample NaN→tooltip 布局 NaN 不画(snap 路径全改 Slide::reset);**平台差异登记**:wasm click 后首发 key 被宿主吞噬(race-chart 同族,验证脚本 Escape 预热)、aria_pressed/sr 表/roledescription 官方有而 GPUI 缺或部分缺。**web 实测 18/18+动效 7/7+单测 8**(rd_verify.py/rd_motion.py,1200×1000 SwiftShader):卡 898×301/svg 居中 221 宽/4 环 maxR160 步40/两系列 12 顶点值映射±3.5px/标签三窗 50/50/29/标题+meta/legend swatch/hover tooltip (664,168)≈官方/盒 159×87/紫上蓝下降序/gap 24px/marker 64px/远标签 dim/键盘顺时针 i0→i1/Esc 清/legend 按压紫 1253→0;动效:展开 tip 半径 34→144→160 单调/morph 顶点 131.5→102.2 无瞬移(max 步 29.2)/tooltip 滑移 (682,368)→(652,452)/离开淡出(pre 有 final 无)/marker i0→i3 垂直滑移 y 463→519→521 单调(单测同步锚定:起点 275、终 373.4、tip 终 246.4、无帧跳>45);**取证发现**:低值轴顶点 marker 会滑入不透明 tooltip 盒下方(官方同公式同 z 序同遮盖,验证脚本避开该轴)。fmt/clippy 4 包/228+106 测试(新增 8)/wasm check 双目标/build-web 绿;中文字体子集已重建。**待验**:桌面原生、Safari/125%/RTL、真触摸、kui add、官方实物像素对照、sr 表/roledescription/aria_pressed 平台补齐。| 部分通过 · `SAVIM□D□` |
| <a id="review-split-bar"></a>30 [占比条](http://127.0.0.1:4321/kui/components/split-bar/)<br>[split-bar](</Users/leon/Documents/Kense ui/KUI/docs/items/split-bar.md>) | P1 | 分段比例/间隙/端帽、颜色、内部标签和图例。 | hover/tap锁定、极小段、零值/总和变化；选中段展开、其他段让位、字宽拟合。 | **2026-11-09 对照官方 split-bar 实物 DOM/CSS/交互/动效实测全量重写**(Pro 无源码;取证 /tmp/kv8/split.md+sp_full.html+sp_css.css+sp_segs/legend.json+6 轮脚本实测,oklch→rgb 矩阵换算)。本轮修改：①API SplitSegment{id,label,value(0/负值剔除),color?}(默认 series1-4+两 neutral)+title/description/format/unit/total_label("Total")/legend/SplitInspect/SplitToggle 事件;②官方色 oklch 换算:series1=rgb(85,173,255)=KUI ACCENT/series2 rgb(156,55,190)/series3 rgb(230,110,0)/series4 rgb(0,168,97)/neutral text-muted rgb(140,140,140)+border-strong rgb(72,72,72);③**oklab 混色精确复现**(官方 color-mix in oklab):sRGB↔linear↔oklab 全矩阵(白点守恒验证),段 wash=42%/active 62%/dim opacity .5,官方 wash 实测 rgb(52,83,113) 与实现逐通道吻合(±2);④bar h44 r12 bg=KUI stage(官方 surface-muted 同值 36),段 clip r4 且**首/末可见段外侧 12px**(官方 bar r12 裁切),段间 2px 缝,内缘 3px 纯色 cap(末段无),边缘 spring 0.4s;⑤inside 三态拟合 full(name+pct)/pct-only/lane(leader S 曲线 cubic (c,0)(c,10)(l,6)(l,15) 1px border-strong≈ink19% over stage,outside top18 gap4);⑥hover 公式实测复现:target=max(share+4.5%, needed%),needed=(name+pct 宽+59px)/bar_w,其余段按比例让位(scale=(100−target)/(100−share)),Organic 272→304px(+4.5pp 精确吻合官方 41.571→46.071);⑦readout 双态:Total=hint14+18/500 千分位滚轮+unit;高亮=纯色 swatch 8×8 r2(官方 var(--seg))+段名 14 secondary+41.6% 18/500+value 14 faint 滚轮;⑧legend pill 32 r10 p6/10/6/8 hover/pressed bg stage,swatch 12×12 r3 border-right 3px 纯色+42% wash,off=hollow 1.5px border-strong+划线+“Off”,aria-pressed 句式,Enter/Space 切换;⑨bar 键盘 ArrowRight/Down/Left/Up/Home/End/Esc+Enter/Space 关高亮段(off 后清高亮,官方同),role Group+官方 aria 句式;⑩demo 官方三期数据(7d 99,300:41.6/24.8/19.1/9.9/3.1/1.5%)全反推(源数值不可见,和=total+share 一位小数吻合;官方 hidden total 89,430→Social≈9,870 vs KUI 9,800,差因 demo 数据不可精确复原)+官方色+卡 720。**平台差异登记**:官方最后可见段不可关时 shake+播报(GPUI 无位移 API)、aria_pressed/live region("Social hidden.")部分缺、roledescription 缺。**web 实测 18/18 判据=静态 11/11+交互 7/7+动效 4/4**(sp_verify/sp_interact/sp_motion.py,1200×1000):静态含六段 oklab wash 逐段吻合/3px cap×5/r12 圆角(top 行 x+4 处仍是 36)/44 高/inside name+pct 文字行 302-313/lane 标签+leader/readout/legend swatch wash+cap;交互含 hover 展宽+dim+readout 切换/Referral 撑宽 16→125px(官方≈135,字体度量差)/ArrowRight×2 走段/Enter 塌缩+legend 恢复/legend 点 Social 塌缩/切 30d reflow(41.6→40.0%);动效帧序列连续无跳变:hover 272→285→291→296→299→301、切期 272→…→265、隐藏 77→47→31→18→9→3→0+邻段 150→171、恢复 0→25→43→56→64→70→73。fmt/clippy 4 包/229+106 测试(新增 oklab wash 单测)/wasm check 双目标/build-web 绿;**取证教训**:headless 无 wheel、CDP key 必须三参合法名(key"Right"单参会变"/")、count region 参数序 (x0,y0,x1,y1)、采样行避开段内文字。**待验**:桌面原生、Safari/125%/RTL、真触摸、kui add、官方实物像素对照、shake/live region 平台补齐。 11-09 补齐颜色助手的安装依赖。独立安装及示例原生/WASM 编译通过。 | 部分通过 · `SAVIM□D□` |
| <a id="review-race-bar-chart"></a>31 [动态排名条形图](http://127.0.0.1:4321/kui/components/race-bar-chart/)<br>[race-bar-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/race-bar-chart.md>) | P1 | 排名/条宽/标签/数值轴、播放栏与时间标记。 | 播放暂停/重启/跳帧、并列名次、进出榜；ID稳定、排序换位和数值滚动。 | **2026-11-09 对照官方实物 DOM/CSS/markdown 全量重写**(Pro 无源码;取证 /tmp/kv7/rb_card_full.html 含 sr 表 10×12 官方数值+rb_css.css+rb.md 全 API/motion/键盘/a11y/响应式)。本轮修改：①API 重构 items(id/label/group)+frames(period,values,缺失=0)+label+**groups 前 4 映射 series 色+legend**(无 groups 全 series-1)+bars=8 截断+highlight+value prefix(compact $14K/$660K/$3.87M,K 档整数/M 档两位小数去零)+formatTick **nice 1-2-5 向下取整**(220K→200K 与官方 ticks 吻合,axis_max=ceil);②卡片 min(100%,720)/gap20/p24(<420 20/16)+title16/500+meta13;③head periodText 30/500+periodWord 交叉淡化(0.16s)+total 13 muted 右基线;④plot h=26+8×36+4、row h36、bar h24 top6 **40% wash+3px 纯色右缘 r0/4/4/0**(overflow_hidden 裁圆角)、rowLabel 104 右对齐 13(<400px 移入 bar 内)、rowValue 端+8 13 tabular、highlight ink 500、未选中 dim 55%、**top-8 截断**(界外停 288 opacity0)+升入 0.26s 淡入+**paint 反序=名次层叠**;⑤ticks 1px+12 muted 标签+baseline 1px 于 bar_left(bar 遮挡段透出=官方行为);⑥tooltip min150 p10/12 r14 tray+title/value16·500/note『Rank n / N』+图内 clamp;⑦play 36 圆(border/hover tray/icon16 Play/Pause/Replay)+timeline rail h4/fill ink55% blend/**stops 2×2 每 period past 态**/thumb16 ring/ends 首末期;⑧键盘 Space/Enter 末尾重播/←→/Home/End/**↑↓ 按名次读条+tooltip**/Esc+last_input_was_keyboard 守卫;⑨live region(period+leader)+sr 表(逐项全期值);⑩demo 官方数据 10 商品×12 月+4 组+highlight chair+$+新中文文案重建字体子集。**两个真实缺陷修复**：**wasm 动画持续 re-render 中 press/release hitbox 失配→on_click 全失效**(hover/mouse_down 正常、静止后 click 正常——逐项隔离定位;play 改 on_mouse_down press 触发,平台缺陷登记,funnel 等静态场景未暴露);**播放 delta clamp 0.05s 在节流帧率下慢放约 6×**(改 0.25,wall-clock 语义)。**web 实测 27/27+动效 5/5**(1200×713 SwiftShader 像素取证,rb_verify.py/rb_motion.py):8 行 h24 全对/行距 36/3px 纯色缘 run=3/**ticks 4 条等距 98px**(479/577/675/773)/baseline 91px 透出段/period 大字 span22/total/legend 4 swatch/tooltip 3 行(289..544 tray)/dim 55%/autoplay 推进 360→412/**play 暂停 415→415**(修复后)/scrub thumb 875≈883/**↑ 读条+移出保持(n=10704)/Esc 清(→2)**/Home 回 1 月 word 宽 105<143;动效:**overtake 相邻帧 max 3px 无瞬移**/**尾部行升起 144px maxhop72**/**replay V 形 18596→9696→16104(扫回+重播)**/word 交叉淡化 mid355<497。fmt/clippy 4 包/213+106 测试(新增 play_button TestApp 点击测试)/wasm check/build-web 绿;tests 显式导入。**待验**:桌面原生、Safari/125%/RTL、真触摸、kui add、官方实物像素对照、**gpui wasm 动画中 on_click 平台缺陷**(组件级绕过已登记,funnel 等静态场景不受影响)。| 11-10 已修复换月抖动，重新验收 · `SA□I□□D□` |
| <a id="review-funnel-chart"></a>32 [转化漏斗](http://127.0.0.1:4321/kui/components/funnel-chart/)<br>[funnel-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/funnel-chart.md>) | P1 | 各级梯形/带宽、接缝/曲率、转化数字、图例和颜色。 | 切换阶段/系列、异常与零值；整体轮廓插值、流动粒子/高亮及离屏暂停。 **2026-11-09 对照官方 funnel-chart 实物 DOM/CSS 全量重写**(Pro 组件无源码,live DOM+computed CSS 取证:/tmp/kv7/funnel.md)。本轮修改：①band 几何按官方反推——plot 高 200/中心线 y100/half=(cross-16)/2=92/厚度=v/max×92,3px 步长 fill-only;②**total 模式分层堆叠语义**——每 segment 一条 path 同 series-1 24% mix(边界重合不可见,官方 CSS `:not([data-view=total])` 才分层配色),split 才用 series-2+ 且 42% mix;③内边界 cums 只取前 layers-1 累计(初版误含外边界→紫层零厚度,像素实测捕获并修复);④ease 窗口 fraction 0.275..0.725 smoothstep(r³(6r²-15r+10)),slot 中心=步中心;⑤split 每层沿堆叠方向平移 ±1px(2px 分离);⑥isolate(data-view=one)选中段 share→1 其余→0,morph 0.42 rescale;⑦glowLayer 同 path 纯 series 色 clip 到 focus 步列 opacity 0.16s+soft column r14 ink 3.5%;⑧flow dots 40 个/每 dot 预决定 exit 边界(draw>kept 离开)/exit 0.12s 下坠 20px 淡出/reduced 与离屏不画;⑨drops 每边界箭头 svg(stroke1.3 round)+整数%;⑩tooltip min-w168/max-w240 p10/12 gap4 r14 tray 底/split 双段行含 10×2.5 swatch/tipMeta 第一步空隐藏/tip_x/y snappy 0.26+clamp 图内;⑪header grid N×1fr pb12,值 18px tabular 滚动(roll 0.4 smooth),rate <10% 一位小数;meta『Last 30 days, 3.6% convert』;⑫卡片 title+segmented 滑块 selection+Split/Flow 胶囊 switch(42×24 thumb18);⑬竖直 <420px:plot 84×n/side rows 右 28%/drops 贴边界;⑭键盘 Right/Down/Left/Up/Home/End/Esc+focus 读第一步+键盘输入 hover 守卫;⑮a11y live region+sr 表;⑯compact(100K/61.2K/2.5M)与千分位(100,000)分场景。**web 实测 29/29+动效 3/3**(1200×1600 SwiftShader 像素取证,fn_verify.py/fn_motion.py):五步厚度 184/112.6/30.2/15.5/6.6 实测 184/112/30/14/6(≤1px);hover glow 2841px 纯色/tooltip title+Total+kept 3 行;split 42% 双层(蓝+紫)gap 4px/tooltip 5 行(Mobile 68,420+Desktop 31,580+meta);isolate mobile 厚度 184/107/25/12/4 全中/All 还原;drops 39/73/49/57%;键盘 click 聚焦 6421px/End 跳末步/Esc 清除(基线 303→310);竖直 390px band 高 319/side rows;reveal swell 单调(6108→45686)/split morph 中途带持续(33822→34311 无重启)/tooltip 淡入(433→12546);console 无错。fmt/clippy(-D warnings)/209+106 测试/wasm check/build-web 绿;tests 显式导入规避 glob 深链 rustc 爆栈。**待验**:桌面原生、Safari/125%/RTL、真触摸、kui add 独立安装、官方实物像素对照。| 部分通过 · `SAVIM□D□` |
| <a id="review-streamgraph"></a>33 [流图](http://127.0.0.1:4321/kui/components/streamgraph/)<br>[streamgraph](</Users/leon/Documents/Kense ui/KUI/docs/items/streamgraph.md>) | P2 | 堆叠基线、曲线平滑、颜色、坐标/图例及边缘留白。 | 筛选/数据范围/悬停、空或缺失值；共享网格插值、路径/提示与快速更新。 **2026-11-09 对照官方 streamgraph.tsx+module.css+motion tokens 全量重写**(registry JSON 取源)。本轮修改：①Fritsch–Carlson 单调三次重采样到 161 共享网格(原线性插值,曲线有棱);②Byron–Wattenberg wiggle 公式修正(below 累积变化量、按层值加权,原实现两项均写错)+**最小二乘去线性漂移**(原缺失,流整体倾斜);③stack_order 官方中位交替堆叠(2,0,1,3;原按序堆叠)+StreamOffset API(Wiggle/Silhouette/Zero);④**指针层命中**(render 内共享帧几何):命中层纯色、其余 34% 透明、tooltip 对应行高亮、图例 hover 预览同链路(原全无);⑤层间 2px surface 描边分隔(官方 stroke 当 gap,原无);⑥direct label held-spot(morph 位置保持至新点厚 80%)+8 向 surface 描边可读(原无保持);⑦时间轴(axis_picks 72px 步距从最新倒数/月标签/26px 轴区,StreamPoint::with_axis_label,原无轴);⑧键盘官方键序(Left/Right ±1、PageUp/PageDown ±n/6、Home/End、Up/Down 换层、Esc 清、focus 开最新点;原 up/down 误作时间移动且无换层);⑨tooltip 行序按视觉顶到底(stack reverse)、active 行 ink+其余 50%、骑层中心、tip_x/y spring 0.26s+淡入 0.16s、surface-raised(tray) 底 r14 min-w156(原固定 top=78/panel 底/无高亮);⑩空态『No data for this range/该范围暂无数据』;⑪图例 pill 官方态(删除线+swatch 6px 35%、focus bg ink5%、dim 56%)挪至图上方;⑫a11y 隐藏表(role Table/Row 逐点逐层);⑬键盘输入 hover 守卫(last_input_was_keyboard,键盘读数不被 gpui 假 hover-false 误清);⑭demo 官方语义(Support tickets by topic/tickets、Bugs/Billing/Onboarding/Account、KUI series 四色、月度轴标 Mar/Apr/May);⑮时长对齐 motion tokens 实测:settle=smooth .4(数据 morph/层隐藏)/swell .9(入场)/glide=snappy .26(crosshair/tooltip)。**web 实测 27/27**(1200×1600 SwiftShader):四层 82% series 混色/首层居中(stack 2,0,1,3)/wiggle 去漂移(中心左右差 12px)/2px 层缝 2656px/hover:层纯色+邻层 34%+crosshair+tooltip 5 行(标题+四层)/图例隐藏塌缩(紫 4930→0)其余重排/恢复(鼠标停留 pill 上为 focus 纯色 4950)/键盘:click 聚焦读数、End 跳最新(tooltip 右缘贴 plot 右缘)、ArrowLeft 一步(700→654=46px)、ArrowUp 换层全亮(account 3526px)、Esc 清除/时间轴月标签/入场 swell 单调(5575→22272,首帧 25%)/console 无错。fmt/clippy(-D warnings)/204+106 测试/wasm check/build-web 绿;新中文文案已重建字体子集(subset-story-font.py,Noto Sans SC VF 重新下载)。**待验**:桌面原生、真触摸、RTL、官方实物像素对照。| 部分通过 · `SAVIM□D□` |
| <a id="review-treemap"></a>34 [矩形树图](http://127.0.0.1:4321/kui/components/treemap/)<br>[treemap](</Users/leon/Documents/Kense ui/KUI/docs/items/treemap.md>) | P1 | 面积/分割算法、层级内边距、圆角、标签阈值和颜色。 | 钻入/返回/面包屑/指标切换、空层级；缩放原点、稳定ID重排、快速返回。 **2026-11-09 对照官方 treemap.tsx 全量重审**(registry JSON 取源+live DOM 取证)。本轮修改：①squarify 分割+官方 layoutFrom 嵌套(顶层 tile gapped GAP/2=1px 缝为官方行为;GAP=2/PAD=3/HEADER=22/TIP=14);②**钻入/退回画面冻结根因修复**——zoom_to 改 center 后 layout 恒旧致动画空转，置 dirty 重算；rebuild 置 layout 空；③rebuild 按 tile id 重映射 drawn/room_slides(metric 切换跨数据 morph 不重播 reveal)，fills_prev 交叉淡化 0.48s=官方 --duration-considered(.48)实测;④键盘导航按官方键序(首按方向键 inspect 最大 tile,Arrow 移动,Enter 钻入,Esc 退回/根级清除);**键盘输入后 gpui hover 恒 false 会误清 active**——on_hover 加 last_input_was_keyboard 守卫(Esc 后 ring/dim/tooltip 保持=官方);⑤r12 舞台圆角:GPUI overflow_hidden 为矩形裁剪,4 角 panel 色 24×24 r12 贴片等效;⑥active ring 2px ink/lineage 1px hue 70%/非谱系 tile 50% dim;⑦tooltip 跟指针+移出隐藏;⑧demo retention 数据错档 25/36/36/47→14/25/47/58(原两度量同量化档,shade 切换零视觉变化,官方『切换度量 morphs every tile』能力不可见)。**web 实测 37/37**(1200×1600 SwiftShader 像素取证):4 分支 9% 头/叶量化档(STEPS=5:14-58 与官方同式)/stage 418px/r12 角裁剪/header band 23px/gap 2px/tooltip 176px≥4 行/钻入 north 蓝 17788/morph 中间帧几何插值/Esc 退回 ring 626px+三分支 dim(头 1608-2332,叶 11805px)/ArrowRight ring west/Enter 钻 west 17649/根级 Esc 清除/metric remorph(几何 441→619)/shade 四分支重分档交叉淡化(mid 20810-32234,旧色残留≤12px)/reveal 大块先出(10332 vs 0)单调/console 无错。fmt/clippy(-D warnings)/306 测试(kense-kui 200+kui-story 106)/wasm check/build-web 绿。**无关修复记录**:loader.rs(morph-loader demo)tests 模块移至文件尾(kui-story --all-targets clippy items_after_test_module,既有提交顺序问题,行为不变);kense-kui --all-targets 既有 lint 债(money_input/phone_input 等测试代码 44 处 needless_borrow/digit grouping)属其他组件,未动仅记录。**待验**:桌面原生、Safari/125%/RTL、真触摸、官方实物像素对照。| 部分通过 · `SAVIM□D□` |
| <a id="review-brush-chart"></a>35 [区间趋势图](http://127.0.0.1:4321/kui/components/brush-chart/)<br>[brush-chart](</Users/leon/Documents/Kense ui/KUI/docs/items/brush-chart.md>) | P1 | 主图/缩略图、选区/手柄、坐标与日期读数。 | 拖选/拖边/整体移动、键盘、最小区间、边界；范围/坐标弹簧与提示更新。 | **2026-11-08 对照官方 markdown 补齐五项**(官方 Arc Pro，公开 markdown+live preview)。KUI 原有实现已含拖窗口/双柄/键盘/双击重置/tooltip。**本轮修改**：①minSpan 7 点(update 中心扩展+drag_to 锚定钳制+柄/缩放钳制)；②窗口键盘 PageUp/PageDown 半窗平移；③柄键盘 Shift×10 点；④空态 `No data yet/暂无数据` 覆盖层+canvas 早退；⑤事件刻度画到 overview 缩略图(2×7px success 竖标)+a11y 隐藏表(role Table/Row 逐点 aria_label)。**web 实测**(1200×1600 SwiftShader 像素取证)：strip 定位 rows 445-496/卡内 x337-861；暗带 0.65×panel=(27,27,27)×420px、可见窗口 (30)×72px、柄 (25)×6px、窗口亮边 ~124 灰 x786-862、overview 线 181 灰(带下 35% 透出为 81 灰)、事件刻度 x827 全部像素定位；点击暗带 (500,470)→新窗口 25-31=预测 484/519 与实测一致；窗口拖拽 778-861→464-550 跨度保持；左柄拖拽 778→683(start 75→58.8 精确)；preset 60 天实测 511 vs 预测 514；键盘 PageUp −7/ArrowLeft −1/End/Escape+0 全范围/plus 从全范围缩进 8 点(预测 384/814 vs 实测 381/817)；Shift+左柄 −57px≈10 点；入场 reveal 画线 8 帧单调 0→103→217→306→355→380→392→396px(~1s)。PageDown 在右缘 no-op=正确钳制(last-width 上限)。fmt/clippy(native+wasm -D warnings)/191 测试绿。**待验**：双击重置 web 合成事件 click_count 不可靠(代码在,官方等价 Escape 已实测)、空态/隐藏表 demo 无法触发仅代码核对、官方实物像素对照(取证 Chrome 环境恢复后)、桌面原生、真触摸、RTL。 | 部分通过 · `SAVIM□D□` |
| <a id="review-ridgeline"></a>36 [山脊分布图](http://127.0.0.1:4321/kui/components/ridgeline/)<br>[ridgeline](</Users/leon/Documents/Kense ui/KUI/docs/items/ridgeline.md>) | P2 | 各分布曲线/垂直间距、轴/标签、填充与前后遮挡。 | 分布切换、峰值/空样本、hover/选择；抬升、曲线插值和提示跟随。 | **2026-11-08 对照官方开源源码逐项重写**(免费组件，registry JSON 取得 ridgeline.tsx+css 全文)。原实现存在多处结构性偏差，本轮修改：①**全局峰值归一**(原按各系列自身峰值归一→所有脊同高；官方按全部系列最高峰归一，宽脊明显更矮，单测断言 spread<0.6)；②Silverman 带宽经验规则+域=数据±2.5 带宽再取 nice step(1/2/5)；③**左右键改为读数光标**(原为换脊，官方 ArrowUp/Down 换脊、Left/Right 沿轴步进、Home/End 首末脊、Esc 全清)；④每条脊常显中位刻度(1px tone 55%，抬起副本为 ink 1.5px)；⑤抬起副本配色按官方：填充 18%、四分位带 48%、轮廓 2.25px、上升 6px、其余脊 0.38 暗；⑥**脊形命中**(按曲线覆盖范围取行、偏好下行，原为固定行带)；⑦tooltip 四行结构 label/Median/Middle half/Range+`P% at or below V` 读数注记，保留内容淡出；⑧轴刻度线(1px quad)+标签 step 5 居中；⑨按中位数的单色 tint(38–100%)与 5 档色阶图例 18×10；⑩空态 `No data yet/暂无数据`；⑪域 lo/hi 与行号 Slide 化(数据切换时脊滑动/膨胀)；⑫focus-out 清除；⑬a11y 隐藏表(中位数/四分位/n)。**web 实测**：刻度标签 -10..40 步 5 共 11 枚列 390-862(预测 392-860)；刻度线 10 列；tooltip 定位与官方公式吻合(638-807/行 290)；Mar 暗化饱和 56.1→24.6≈0.44×；键盘 ArrowRight 十字线 577→624=+47px 精确一步；Esc 清除；morph f1 为中间态(与终态差 19028px)后收敛；reveal 蓝像素 2122→3421 单调。**平台边界**：gpui 无 aria_live(登记)；wasm 长竖直 stroke 不渲染→刻度线/十字线改用 1px quad(已修)；fmt/clippy -D warnings/194 测试/wasm check 绿。**待验**：官方实物像素对照、桌面原生、真触摸、RTL、tooltip 淡出帧序列。 | 部分通过 · `SAVIM□D□` |
| <a id="review-realtime-stream"></a>37 [实时数据流](http://127.0.0.1:4321/kui/components/realtime-stream/)<br>[realtime-stream](</Users/leon/Documents/Kense ui/KUI/docs/items/realtime-stream.md>) | P1 | 时间窗/坐标、曲线/面积、最新值与控制栏。 | 暂停/恢复/重连、窗口滑动、缺点/乱序/空数据；路径平移、点入场和离屏时钟。 | **2026-11-07 对照官方 markdown 对照修改并验证**(官方为 Arc Pro 组件，源码不可访问，仅有公开 markdown API 表与公开 live preview)。已实现核对：图高 200、hover hold(pauseOnHover)、空态文案 "Waiting for data"、anomaly 标签 pop overshoot(t+0.3sin(πt))、值轴 Slide ease、Live/Paused 状态按钮形状区分(圆点/双竖条)+aria_toggled+Space/Enter、离屏停帧。**本轮补齐**：①Tab 聚焦即冻结并读最新点(原为释放，按官方键盘表改)；②PageUp/PageDown ±10 点(单测验证 selected 步进；wasm 层 CDP 发键浏览器已收到但组件未动，疑 GPUI web 平台派发问题，逻辑层已有单测背书，待查)；③恢复时从冻结视图 glide 滑回 live(shift.translate(1.5)+0.45s，原为 reset 跳变；web 实测 Esc 后帧差 886=连续滑动)；④a11y 隐藏表最新 20 条读数(role Table/Row)。web 实测：滚动帧差 1146、hover 冻结 0、ArrowLeft 选中 321、Esc 恢复 886；fmt/clippy(native+wasm)/192 测试绿。**API 模型差异登记**：官方 windowMs/interval 时间窗+60fps head 插值 vs KUI capacity 缓冲+push 时 0.35s 滑格(视觉等价，节奏 240ms≈0.35s)；formatValue/formatTime/min/max props 未建模。**待验**：官方实物像素对照(取证 Chrome 环境恢复后)、V/M 阶段、桌面原生、真触摸。 | [截图返修与独立安装已验证](#用户截图专项复查2026-11-09) · 待完整对齐验收 · `□□□□□□D□` |
| <a id="review-sunburst"></a>38 [旭日图](http://127.0.0.1:4321/kui/components/sunburst/)<br>[sunburst](</Users/leon/Documents/Kense ui/KUI/docs/items/sunburst.md>) | P1 | 每层内外半径、角度/间隙、弧标签方向、中心数字和面包屑。 | 钻入/退回/键盘、稳定ID、非法值；弧域重映射、中心读数和快速反切。 | **2026-11-08 对照官方 markdown 补齐两项差异**(官方 Arc Pro，公开 markdown+live preview，源码不可访问)。KUI 实现原本高度完整(键盘←→↓↑/Enter/Esc/Backspace、面包屑、图例 value+share+hot、中心滚动计数、tooltip、a11y 隐藏表、弧插值)。**本轮修改**：①空态中心文案 `No data yet/暂无数据`(官方 emptyLabel，total≤0 时显示，替代数字)；②聚焦高亮首段——图表获得焦点时 active 自动设为首个弧(官方「Tab focuses the plot and highlights the first segment」；实现走 `cx.on_focus` 订阅+track_focus，Stateful<Div> 无 on_focus 方法)。**web 实测**：几何 340px/外径 168/内孔 60/暖蓝紫青系列色齐全；点击中心聚焦后首段保持 0.96 不透明度、其余弧降至 0.42(warm 区亮像素 20305→2332、blue 亮度 0.559×)；Enter 缩放弹簧连拍 f0(16986/32303)→f4(39243/37502) 渐变收敛 ~0.8s(0.62s Slide+ease-out)、缩放后布局 diff 69785px 证实钻入；Esc 返回 0.4s 时 diff 18517px=回摆中途非跳变。fmt/clippy(native -D warnings)/wasm check/190+106 测试绿。**平台边界登记**：GPUI 无字形旋转→弧标签保持直立(官方沿弧旋转)。**待验**：官方实物像素对照(取证 Chrome 环境恢复后)、V/M 阶段、桌面原生、真触摸、RTL。 | 部分通过 · `SA□I□□D□` |
| <a id="review-sankey-flow"></a>39 [桑基流向图](http://127.0.0.1:4321/kui/components/sankey-flow/)<br>[sankey-flow](</Users/leon/Documents/Kense ui/KUI/docs/items/sankey-flow.md>) | P0 | 先同官网核对整体布局：节点/列位置、带宽比例、留白、颜色/透明度、标题和标签。 | 悬停上游/下游追踪、流带命中、提示/固定、周期切换；带形变、粒子密度/速度与连续中间帧。 | 用户指出完全不同，已按公开文档与示例重写。**2026-11-07 实现并逐项验证**（wasm SwiftShader 像素取证）：①布局几何对齐官方——列距实测 182px(官方 182.73)、源列节点条 y/h 149/143·306/107·424/57(官方 4/143·161/107.25·282.25/57.75)、下游列内间距 207/120/77/147(官方 206.78/120.15/77.52/147.92)、修正列内为**底对齐**(四列底缘统一 485)与 scale `(minor-4-gaps)/widest`(原-8/顶对齐)；②头部 meta `11,200 signups, Jul to Sep 2026`、列头 16/22.4 与列心对齐、源列实心系列色+下游 `oklab(0.7514)` 灰、rx2.5、Q2/Q3 按钮右上、Flow 胶囊 br9999 右下(方点 on 227/off 170 亮度切换)；③动效：首现 0.4s/Q2 切换 0.42s morph 实测中间帧 143→146→147(官方 Q2 organic 147.58)、粒子帧差 194→off 后 0(停止请求帧)、修复**粒子仅在选中时显示**的缺陷(现无选中时全带流动)；④交互：hover 带 focus 0.36/其余 0.05+tooltip 卡出现/静止保持/移开消失、点击聚焦后 ArrowRight×2 选中切换(diff 6469/4667)、Esc 恢复(7323)；⑤demo 容器 `w_full` 修复 flex 收缩导致的竖排误触发；⑥fmt/clippy(native+wasm -D warnings)/191 测试全绿，docs/items 同步 `with_total_label/with_period_details`。**待验**：桌面原生(本机缺 Metal 工具链)、Safari/320px/125%/RTL、真实触摸、独立安装复验、官方 hover glow 0.34 叠加层与 nodeGlow(现为直接 0.36 换色，视觉近似)。 11-09 复核发现整带高亮掩盖了部分追踪，已改为按比例绘制并从中间帧回退，专项回归通过。其余状态需按新版本补验。 11-09 同场景验证了比例高亮、提示三行及 Esc，独立安装双目标通过。320px 英文样例 Pro 和 Team 标签间距已修复并复拍，其他窄屏状态待验。 | 参考与分发核对 · `S□□□□□D□` 11-10 补齐手机档标签限宽与避让。见[窄屏实拍](#桑基图窄屏标签专项--2026-11-10)。 |
| <a id="review-metric-ticker"></a>40 [实时指标条](http://127.0.0.1:4321/kui/components/metric-ticker/)<br>[metric-ticker](</Users/leon/Documents/Kense ui/KUI/docs/items/metric-ticker.md>) | P1 | 指标列宽、数值/单位/涨跌符号、图标、分隔和趋势小图。 | 数据刷新/排序、长短数值与缺失项；等宽数字进位、涨跌变色、重新排列与离屏暂停。 | **2026-11-08 对照官方 markdown 补齐**(官方 Arc Pro，公开 markdown+live preview)。KUI 原有：odometer 逐位 25ms 错峰滚轮、flash 变色、面板 sparkline、Tab/Arrow/Home/End/Enter/Esc。**本轮修改**：①换指标时面板保持打开(glide，原每次重开)；②Tab 回到最后聚焦指标(last_selected)；③compact 状态重建计数器 16/18px；④flash 箭头自移动方向滑入(↑ 下偏 3px/↓ 上偏)+ring 扩散(canvas 1→6px 边框 8→13 圆角 0.9→0 透明度 0.45s)；⑤repeat 尾部 28px 渐隐 feather(线性渐变 quad，不挡命中)；⑥面板 grow 高度 58→164 开场；⑦caption 默认 `Last n updates/近 n 次更新`；⑧cells aria 含箭头+数值。**web 实测**：flash 三态 resting→wash+tone+ring→resting 时序；ring 画在洗底色外 1px(rows 87/110)；面板打开 grow 帧序列 382→8226→9055→9138→9221 连续；Escape 关闭 ✓；320px 视口 marquee 漂移 ✓、hover 缓停两帧 0.8s 分布完全一致 ✓、feather 边缘文本均值 133.6 vs 中部 156.0 ✓。fmt/clippy 双包/301 测试绿。**平台边界**：GPUI 无 aria_controls/aria_hidden(登记)；面板 morph 缩放受 div 无 scale 限制(高度 grow 近似)。**待验**：Arrow 换指标面板 glide 帧序列、panel 248px 上限实测、官方实物像素对照、桌面原生、真触摸、RTL。 | 部分通过 · `SAVIM□D□` |
| <a id="review-activity-rings"></a>41 [活动圆环](http://127.0.0.1:4321/kui/components/activity-rings/)<br>[activity-rings](</Users/leon/Documents/Kense ui/KUI/docs/items/activity-rings.md>) | P0 | 先确认官网是连续弧还是刻线表现；核对环数/半径/厚度/间距、端帽/起点、颜色渐变、日期栏和图例。 | 切换日期、hover/tap选环、中心读数、达标与超额；入场错峰、跨100%接续、数值/环同步和快速反切。 | 用户指出差异很大，已按官网 56-tick mask 机制重写。**2026-11-07 实现并逐项验证**（wasm SwiftShader 像素取证）：①几何全对——三环中心线 r99.5/79.5/59.5(实测 tick 带 94-108 主峰 96-104、r2 峰 76-84、r3 峰 56-64)、tick 3 点起 6.43° 间隔、**二值显隐**实测 41 段止于 257.8°(理论 40.4×6.43=257.1°)；②三枚 marker 径向指针实测位于 progress×360 角度 (323,249)/(343,216)/(401,288) 恒亮；③中心常显 `69 % of goals`(三环均值 0.6852)36px wheel，修复**入场数字不滚动**缺陷(`with_animate_on_view`，实测 155→143→112→327 像素序列=0→69 滚轮+逐位错峰)；④入场环展开实测渐进(12→27→30 ticks @0.31/0.51/0.7s≈0.4s smooth)；⑤图例三行间距 58(官方 56)、周按钮 7 枚等距 82px、**默认选中最后一天 Sun**(背景 48 vs 25)、day 切换 30→42 ticks 接续无闪白、中心数字同步滚动；⑥fmt/clippy/191 测试绿、docs/items 同步 `new` 签名(cx)与动效说明。**待验**：桌面原生(本机缺 Metal 工具链)、Safari/320px/125%/RTL、真实触摸与读屏、跨100%第二圈(>100% 数据)实拍。 11-09 复核发现切日误触发达标、缺少窄屏几何及游走焦点，已修改并通过专项回归。其余状态需按新版本补验。 11-09 测到真实 Tab 停在所选日期，箭头随后移焦。独立安装及示例原生/WASM 编译通过。 | 参考与分发核对 · `S□□□□□D□` |

<a id="media"></a>

## 文字与媒体（13 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-text-reveal"></a>42 [文字揭示](http://127.0.0.1:4321/kui/components/text-reveal/)<br>[text-reveal](</Users/leon/Documents/Kense ui/KUI/docs/items/text-reveal.md>) | P2 | 词/行分段、换行、字号/字距和最终基线。 | 延迟/重播/换文本；逐词错峰、位移/透明度/模糊、收尾和减少动效。 | 查真实多行、中英混排与快速更新，不只录短单词。 | 待重审 · `□□□□□□D□` |
| <a id="review-in-view-title"></a>43 [视区标题](http://127.0.0.1:4321/kui/components/in-view-title/)<br>[in-view-title](</Users/leon/Documents/Kense ui/KUI/docs/items/in-view-title.md>) | P2 | 标题分行、溢出裁切、起止位置和页面留白。 | 滚入/滚出/返回、once与重播；触发阈值、延迟和入场轨迹。 | 需用实际页面滚动触发，不能仅重新加载录入场。 | 待重审 · `□□□□□□D□` |
| <a id="review-text-morph"></a>44 [文字形变](http://127.0.0.1:4321/kui/components/text-morph/)<br>[text-morph](</Users/leon/Documents/Kense ui/KUI/docs/items/text-morph.md>) | P1 | 相同/变化字符定位、字号/字距、宽度和基线。 | 长短句/中英文/多行、快速反切；匹配字符移动、新旧层交叉淡入、最终布局稳定。 | 查动画结束是否切换排版，防止重复此前数字抖动。 | 待重审 · `□□□□□□D□` |
| <a id="review-text-shimmer"></a>45 [文字微光](http://127.0.0.1:4321/kui/components/text-shimmer/)<br>[text-shimmer](</Users/leon/Documents/Kense ui/KUI/docs/items/text-shimmer.md>) | P2 | 亮带宽度、底色/高亮色、文字边缘和裁剪。 | 开始/暂停/恢复/隐藏；扫过方向、周期、速度、相位连续和减少动效。 | 核对是否真正作用于字形，而非整块背景渐变。 | 待重审 · `□□□□□□D□` |
| <a id="review-slot-text"></a>46 [数字滚轮文字](http://127.0.0.1:4321/kui/components/slot-text/)<br>[slot-text](</Users/leon/Documents/Kense ui/KUI/docs/items/slot-text.md>) | P1 | 数字列宽、符号/小数/分组符、单位和基线。 | 进位/借位/跨零、快速更新和长度变化；滚轮路线、错峰与收尾。 | 必须查静止帧与运动帧同一排版，不能只判断数字最终正确。 | 待重审 · `□□□□□□D□` |
| <a id="review-image-compare"></a>47 [图片对比](http://127.0.0.1:4321/kui/components/image-compare/)<br>[image-compare](</Users/leon/Documents/Kense ui/KUI/docs/items/image-compare.md>) | P1 | 图片缩放/对齐、分隔线/手柄、标签、容器比例。 | 鼠标/触摸拖动、键盘/Home/End、容器缩放；直接跟手与键盘弹簧、边界命中。 | 检查纵向手势滚正文、图像不因切边移动或失真。 | **2026-11-09 对照官方 image-compare 实物 DOM/CSS/computed/交互全量重审重写**(Pro 无源码;取证 /tmp/kv10:ic_full.html+ic_rules.css+ic_styles.txt 暗色实测+ic.md 全 API/键盘/动效文档,另 CDP 真键实测官方方向语义:ArrowRight=valuenow−1、PageUp=+10 after、Home=全 before、End=全 after)。本轮修改:①分割线由 div(panel 底色)改 **canvas PathBuilder 矢量**:2px 线色 dark=ink(242)/light=tray(官方 dark=foreground/surface-raised,暗 48 精确、亮 255 vs 244 登记差异)+3px 黑14% 底层描边=0.5px halo 环、overscan W+H 由 frame 矩形裁剪;②chevron 由文字字形 ‹›⌃⌄ 改 **矢量双箭头**(lucide 16/24 实测点位 (±1,±4)/(±5,0) 相对 handle 中心,stroke 1.75,swing 时绕 handle 中心旋 θ);③handle bg panel→**tray**(dark 48=官方 surface-raised oklch .24 精确)+border line(43=官方 border)/hover faint+shadow 黑22% 0 8 22+focus 时内框 ring 55%;④新增 **set_orientation/orientation() swing**:θ=90°·s 弹簧 0.45s,弦几何 chord_half=min((W/2)|dx|⁻¹,(H/2)|dy|⁻¹) divider 沿弧扫过中心、clip/cursor/chips 即时切;⑤新增**边缘 bump**:边值按键 Slide 1→0 0.24s、handle 沿轴 6px(t=0.19s 实测帧 +4px 后回);⑥键盘清理:horizontal 只答 left/right、vertical 只 up/down、PageUp/Down ±10%、Home 1/End 0,方向语义与官方 CDP 实测全部吻合,aria『N% after』=100−value·100 同官方 valuenow;⑦新增**释放沉降**:拖动 track_move 双采样(0.25s 窗),release 带 velocity(clamp ±4/s)settle 0.3s(实测 794 过冲→761 落定);⑧**touch 轴锁**:Started 不抢,6px 阈值判主轴,与 divider 轴不符放行滚页,tap(无位移 Ended)跳转,dragging 才 preventDefault;⑨chips line_height 20→17(官方 22.8=17+6)、fade 公式保留;⑩移除自创 soften 边缘让位(官方无,handle 贴边由裁剪露半圆);⑪**图片圆角**:gpui img corner_radii 上弧生效下弧不裁(wasm 实测)→四角 8px 填充遮罩 canvas(stage 色多边形 8 段/角,z 最上盖 chips/ring),视觉=官方 overflow clip;⑫frame bg=stage(官方 surface-muted 同值 36);⑬demo 官方化:figure min(100%,540) gap12+bar(『Living room』14/500+『Raw capture and final grade』12 faint)+SegmentedControl Side by side/Stacked→set_orientation+Center 按钮 face 44/r8/p0-12/gap6+rotate-ccw 16px canvas 矢量弧 icon,label『Living room, raw and graded』。**平台差异登记**:radius-panel 26px→KUI surface 8px(用户 8px 硬约束优先)、aria-orientation GPUI 缺、按钮 active scale .97 无 transform、capsule 拖拽中不随 swing 旋转(demo 不触发)、线端 8px 在 divider 0/1 时露圆角缺口、img 圆角下弧平台不裁。**web 实测 19/19=静态 5/5+交互 9/9+动效 5/5**(ic_verify/ic_interact/ic_motion.py,1200×1000 SwiftShader):静态含 540×360 3:2 四角 8px 裁剪+圆内图色/线双列 242 满高+halo 两侧变暗/handle 40 tray 盘+chevron 簇+阴影 below 变暗/chips Before 左上 After 右上/bar 标题 meta+Center 右缘 1px 44 高+icon 最左 ink+segmented 双态文字;交互含拖动 762±3/3 键 +16.2±3+PageDown +54±3/End 330/Home 869 两侧图亮度差>8/双击回中 599/边缘 bump run(18,851)→(14,855)→(18,851)/Stacked 横线 275+After chip 移左下+Before 留/Home 底 455+Center 回 275/切回 Side by side 再拖 761;动效含键盘弹簧中间帧 438→330/释放过冲 794→settle 761/capsule 跨度 52→61→52(chevron 切 run 改 tray 端点跨度法)/chips 淡出 0→恢复 81/swing 帧0 斜 29px→帧2 横满。fmt/clippy 4 包/233+106 测试(新增 chord_half 两轴+45° 弦、chevron 四分之一转对称 2 测,删 soften 测)/wasm check 双目标/build-web 绿;中文字体子集已重建(客厅/并排/上下/居中/原始拍摄/最终调色/成片)。**取证教训**:click 聚焦会同时跳 divider(键盘测试须点当前线位)、CDP count=2 双击在远离 divider 处不触发回中(官方同判定,验证先单击贴近再双击)、segmented 灰字簇须分簇取最右(Stacked)/最左(Side by side)且切换后选中态互换、bar 文字带实测 487-510 与预估不同、验证脚本绝对/相对列坐标混用是最大坑(三轮 M2/M3/M5 假阴性均由此)。**待验**:桌面原生、Safari/125%/RTL、真触摸、kui add、官方实物像素对照、aria-orientation 平台补齐。 | 部分通过 · `SAVIM□D□` |
| <a id="review-carousel"></a>48 [图片轮播](http://127.0.0.1:4321/kui/components/carousel/)<br>[carousel](</Users/leon/Documents/Kense ui/KUI/docs/items/carousel.md>) | P1 | 图片裁切/间距、邻页露出、分页/箭头及图注。 | 拖拽/抛掷/键盘、首尾/循环、尺寸变化；捕获手势、吸附/回弹、自动播放暂停。 | 官网有的示例和交互逐一对照，不能只实现左右换图。 | **2026-11-09 对照官方 carousel 实物 DOM/CSS/computed/交互全量重审重写**(Pro 无源码;取证 /tmp/kv11:car.md 全 API/键盘/动效文档+car_full.html+car_rules.css+car_styles.txt/car_nodes.txt,另 CDP 四轮实测:键盘方向/Home/End/Tab 序、拖动跨卡连续插值与 ≈0.8:1 滞后、轮播 drain 0.2/s 从右排空切换瞬间归零、播放钮 DOM click 与 play/pause icon 叠放)。本轮修改(registry/src/components/carousel.rs 全量重写 ~720 行):①viewport 官方 r26→**KUI 8px**(用户硬约束),track 语义=步进 slide+16(demo 272)、padding-inline (W−slide)/2 中心锁定;②静止非激活一律 **scale .9+opacity .5**(官方实测,非线性衰减),拖动/切换跨过的卡连续插值 opacity 1−0.5|d|、scale 1−0.1|d| clamp 1;③gpui 无 transform→**布局尺寸缩放近似**(卡宽高/字号/行高×scale,caption 高 37.8/20 二值);④photo 1:1 bg surface-muted(=stage 36)+官方 ::after 1px border-subtle→**卡层角遮罩 canvas**(stage 色四角多边形,弧点 (cx+sx√(r²−d²),cy+sy·d) 8 段/角,完整四角高度感知,r=8);⑤caption 官方 grid [1fr auto]:name 14/500 lh15.4 ink+price 14 tabular 行盒 20+detail 12 lh16.8 faint 跨两列全 ellipsis;⑥indicators:role TabList『Choose a slide』,tab 36 高 role Tab aria_selected+tab_index(激活 0),dot 6px 灰 53/hover 183,激活 **dot_in 6→22 pill** 双 Slide(dot_in/dot_out+prev_index,0.3s),**dotDrain** 右对齐 overlay hover 色随轮播进度从右往左排空、切换瞬间归零;⑦controls 行 gap8 px=(W−slide)/2:播放钮(有 interval 时)36 圆 border line bg panel hover tray+play/pause 双 icon 叠放,aria『Start/Pause slide rotation』;arrows 同盘,首/尾 opacity .5+faint 语义=官方 aria-disabled,Tab 序 viewport→play→激活 tab→arrows;⑧键盘 ArrowLeft/Right/Home/End(root Region on_key_down);⑨轮播 interval 5s:spawn+timer+ticket 防重入,hover/focus/drag **hold 冻结**(frozen 快照+from=now−frozen×interval 重推保进度),wrap 回第 1 张,drain 排空即切+restart;⑩拖动 begin(focus+OpenHand→ClosedHand)/move **1:1 跟手**/end 0.12s 速度窗 carry=speed×0.199/step、flick>360px/s 保底一张、release(clamped,−speed/step,0.75) 临界阻尼弹簧(实测过冲 340 后落 477);⑪touch 轴锁 pan-y hitbox 保留;⑫API:new/with_index/with_slide_width/with_interval(≥500ms)/autoplay/with_action_labels/with_rotation_labels/index/is_rotating/set_index(clamp+动画+emit CarouselChanged)/set_rotating。⑬demo 官方化:5 商品(room 图+中英文名/detail/price)+『Furniture and objects 家具与器物』+min(100%,720) 容器+with_index(1) 256px 5s。**平台差异登记**:gpui 无 transform scale(布局近似,非激活 caption 字号同步缩)、官方拖动 motion spring ≈0.8:1 滞后 vs KUI 1:1(文档字面语义)、visibilitychange/IntersectionObserver 离屏暂停平台无 API、reduced motion 直跳禁轮播走 disabled_motion、img corner_radii 不可靠(遮罩)、radius 26→8(用户硬约束)。**web 实测 19/19=静态 5/5+交互 9/9+动效 5/5**(car_verify/car_interact/car_motion.py,1200×1000 SwiftShader):静态含激活卡 256 宽+邻卡 step 272/gap 30/inset 14/scale .9、四角 8px 遮罩+图内亮、caption name ink 125 像素+price 53+detail 353(subpixel RGB≤30 兼容)、指示器一 pill 22+四 dot 6、控件三盘 36;交互含 Next 前进/tab 点击跳转/键盘 ArrowRight+End+Home/prev 首张 dim(95 vs 178)且无效/拖动 caption 477→385(shift 92)/flick 落 579.5/播放 drain 4→11 且 5s 切卡/hover 冻结 2→2→移开 4/停止归 0;动效含切卡弹簧帧 [464,310,240…] 连续非跳/dot 拉伸中间帧 ink 8-9+旧 dot 灰 20→7/右拖跟手 shift 63/释放慢拖 mid 419→final 477 回弹/drain 5→11 递增。fmt 本包/clippy(carousel.rs+media.rs 零警告;**无关失败**:money_input.rs:682 reversed_empty_ranges 属其他工作未动)/测试 234+106(新增 captions 测试)/wasm check 双目标(kui-web+kui-story --lib --no-default-features)/build-web 绿;中文字体子集已重建(家具与器物/橡木休闲椅/乳白台灯/石洗亚麻毯/棱纹玻璃水瓶/暮色海景/编织坐面等)。**取证教训**:激活 pill 中心=531.5+16×index 等距、非激活 dot 距激活 pill 24/40/56/72(点击坐标随激活态变)、drain overlay 在 ink 右侧(从 ink 向左扫必得 0)、CDP 单步 mouseMoved 即 >360px/s 触发 flick 保底(慢拖须两步+停顿)、dot 收缩簇是非激活灰 53 非 ink、整页截图 ~0.7s/张须 clip 小窗连拍。**待验**:桌面原生、Safari/125%、真触摸、离屏暂停 API、官方实物像素全对照。 | 部分通过 · `SAVIM□D□` |
| <a id="review-card-stack"></a>49 [卡片堆叠](http://127.0.0.1:4321/kui/components/card-stack/)<br>[card-stack](</Users/leon/Documents/Kense ui/KUI/docs/items/card-stack.md>) | P1 | 层叠位移/缩放/旋转、卡片阴影、图像和决策按钮。 | 拖动阈值、方向判定、取消、撤销、用尽；顶卡出场、后层前进及快速连续操作。 | 核对中间形态与命中区域，不以硬删除卡片替代。 | **2026-11-09 对照官方 card-stack 源码级全量重审重写**(取证 /tmp/kv12:官方 card-stack.tsx 源码 20415 字节(VISIBLE 4/PEEK 12/SHRINK .05/LIFT .4/ROTATE .065/FLICK 500/FREE_Y 48/STRETCH_Y 140/throw .5s/fadeOut .22s/depth spring .35s/undo .4s/发牌错峰 .06s 全公式确认)+card.md 官方文档+card_rules.css+card_nodes.txt+card.json+CDP 九轮实测 ev1-ev9(几何 320×392、层叠 27.2×i、scale 线性、stamp 16-96 渐显、慢拖释放回弹、键盘决定、undo 飞回、空态样式、Start over、status 文案)。本轮修改(registry/src/components/card_stack.rs 全量重写,502→约 1050 行):①StackCard 新结构 id/code/date/title/subtitle/pitch/facts+StackFact{glyph,text},with_meta/with_pitch/with_fact;②slot_geometry:scale 1−0.05×depth、y 下探 12+15.2=27.2/步(底边逐层 +12 peek)、x=W×0.025×depth、presence (3−d) clamp(第 4 张 opacity 0 待升起)、shade (d/2) clamp 黑 22%;③官方按钮 r18 胶囊、stamp pill、卡面 r34→**KUI 8px**(用户硬约束);④per-card depths/dx/dy/op 四组 Slide:decide 深度 duration .35+飞出 .5+fade .22、undo 自 exits 记录点飞回 .4+淡入 .15、reset 发牌 delayed (count−1−j)×.06 深到浅;⑤拖动 y 轴 resistY/unresistY 橡皮筋(48 自由区/140 渐近)、velocity 80ms 窗首尾差、release 判定 flick(abs(v)>500 且同向且位移>12)或 far(位移>min(W×.35,140) 且反向速度≤300)、回家 morph 弹簧带释放速度;⑥拖动中下层 depth set −lift×.4(lift=hypot(x,y/2)/160)跟手升起;⑦stamp:keep 左上 success 混色、pass 右上中性,opacity=clamp((abs(x)−16)/80) 单向 ramp 各自方向;⑧键盘 ArrowLeft/Right/Backspace/(meta/ctrl)+z;⑨空栈 empty 视图(304 边框卡+标题 16+chosen 名单 join+Start over 36 边框钮 undo icon)入场 y8+opacity .2s;⑩控件三钮 44 高:Skip(border panel)、Undo(quiet 无边框 secondary+disabled opacity .42)、Choose(bright 主底);⑪事件 StackDecision/StackUndo/**StackReset**+message 播报('{title}: {label}. {n} left.'/'restored.'/'All cards are back in the stack.');⑫Glyph 新增 Plane/Wallet(lucide 折线近似);⑬apps/story media.rs demo 6 卡官方数据(LIS/CPH/OPO/KIX 实测文案+EDI/SVQ 官方未渲染自拟,登记差异)+全中文;docs/items/card-stack.md 同步;字体子集重建(里斯本/哥本哈根/波尔图/京都/爱丁堡/塞维利亚/葡萄牙/丹麦/日本/苏格兰/西班牙/晴间多云/每人…起 等)。**平台差异登记**:gpui 无 transform rotate(官方 rotate=x×.065×tilt 卡片倾斜,KUI 以平移近似)、无 aria-roledescription/live region(status 文案经 message() API 提供)、empty 无 blur 入场、throw fadeOut 无 .12s delay(Slide 直跑)、无 img corner_radii。**Web 实测 39/39**:静态 18/18(顶卡 304×320(含 1px 边框口径 ±2)、层叠底边 +12/+24 且第 2/3 张 inset 8/16、第 4 张 presence 0 不可见、meta 行 faint 12px、title 36px 白字带、facts 分隔线+三行簇、静止无 stamp 绿、控件行 44 高(choose 实底 641-751、skip 边框 446-531)、shade 深度暗化 22/19、stage 底 36);交互 15/15(Skip 按钮决定动画 d2596+换卡 d1821、Undo 控件恢复 d0、ArrowRight 决定、Backspace 撤销 d0、meta+z 撤销 d0、慢拖 +60 跟手左缘 441→501、释放回家 d0、stamp 渐显 5 绿像素→拖回 0、快速 flick +48 决定、连点 6 次空栈 empty 顶边线 280 且无层叠 peek、控件 dim、Start over 定位 (599,270) 点击重发 d0);动效 6/6(飞出帧绿 stamp 567→601 递增、下层顶边 98→80→76→75 弹簧连续、拖动两步 471/501、释放回弹 496→459→445→442→441 单调无跳、undo 飞回 442→593→469→446→442 且 face 25→242→25 淡入)。门禁:fmt 本包绿/clippy(card_stack/media/glyph 零警告;**无关失败**:money_input.rs:682 reversed_empty_ranges 属其他工作未动)/测试 236+106(新增 slot 几何/stamp ramp/resistY 圆往返测试)/wasm check 双目标绿/build-web 绿。**取证教训**:wasm 下 BackgroundExecutor.now().elapsed() 时间戳恒≈0 会使 80ms 速度窗失真(任何释放都判 flick),必须存 Instant 以差值计算;CDP clip 截图须做列偏移;层叠卡的可测特征是底边差=PEEK(12)非顶边差;demo 面板 bg 与卡同色 25 须按行结构区分;半透明卡边框混色容差需放宽至 ±10。**待验**:桌面原生、Safari/125%、真触摸、rotate 倾斜(平台不支持)、aria live region。 | 部分通过 · `SAVIM□D□` |
| <a id="review-product-gallery"></a>50 [商品图集](http://127.0.0.1:4321/kui/components/product-gallery/)<br>[product-gallery](</Users/leon/Documents/Kense ui/KUI/docs/items/product-gallery.md>) | P1 | 主图/缩略图、颜色/尺码、价格/按钮、放大镜与查看器。 | 换图/缺图、可用尺码、缩放/关闭/焦点；图片淡换、放大镜跟随和布局稳定。 | 核对实际资源、禁用组合及窄屏溢出。 **2026-11-09 对照官方 product-gallery 规格全量重审收尾**(Pro 无源码;取证按 docs/items/product-gallery.md 官方 catalog/markdown/DOM-CSS 实测:结构 thumb64×80/pitch88、swatch32/pitch44、size92×44/pitch100、lens176、zoom2.4、FLIP .3/.26、TRACK .6、GLIDE .3、FADE .24、售罄 meta 条件渲染、官方 viewer 不可开仅文档规格)。上轮已完成组件主体重写与 viewer 三重修复(anchored overlay 显式视口尺寸+contain 4/5 fit、counter/hint/close 挂 overlay 层手动定位、overlay relative);本轮修复三处交互缺陷并全量验证:①**拖动后 click 误开 viewer**——end() 在 mouseUp capture 阶段先 take(drag),click 冒泡到达时 `drag.is_none()` 恒真,任何 flick/慢拖结束即全屏 viewer(scrim 盖组件造成『indicator 消失/换色失效』假象);改为 Drag.dragged 标记(位移>3px 置位、begin 清零、on_click 消费一次),纯点击仍开 viewer;②**viewer 关闭后页面永久失活**——viewer_leaving 靠 280ms BackgroundExecutor timer 清除,wasm 上后台 timer 不触发,透明 overlay 持续 occlude() 吞掉所有 wheel/点击;改用 bottom_sheet 同款 Slide 收敛判定(viewer_open.value≤0.001 即卸载),删 timer 与 viewer_leaving 字段;③**demo 购物袋计数不刷新**——footer 按钮闭包仅 &mut App 无法 notify 组件,Rc<Cell> 计数永不重渲染;改独立 BagPill Entity(update+cx.notify,作 child 嵌入 header,on_click 直接更新)。**Web 实测 71/71**:静态 20/20(stage 573×716、四 thumb 64×80、ring 68×84 环、title 28px、bag pill 32、三 swatch 色+32 宽环、五 size 簇 pitch 100±7、Cal king 划线、add 50 高、meta 行);交互 41/41(flick 进一、thumb 点选、慢拖回弹、lens 出现、viewer scrim 盖页/contain 照片/counter/标题遮挡、zoom 往返、viewer 方向键+escape+照片同步、stage 方向键、King fill、forest 环+清空+划线+meta 重述+rail3、售罄拒选、queen fill、walnut 环+保留+meta 消失+rail2、cal-king fill+回 oat 清空+划线+rail4、add 计数 pill 差分 74);动效 10/10(释放弹簧 diff-to-rest 0.328→0 单调收敛、ring glide 中间行 292→368、3→0 中途反切到 2 自当前行 255→280 接续、换色无闪帧首帧居间、viewer scrim title-ink 1072→773→0 渐深、zoom diff 0.482→0.631 中间态、fill 反切 465→620 连续滑动)。门禁:fmt 本包绿/clippy 双包零警告/kense-kui 239+story 110 测试绿/wasm check 双目标绿/build-web 绿。**取证教训**:mouseUp capture 与 click 冒泡的时序使 drag 状态守卫失效,拖动类组件必须在 click 侧消费『刚拖过』标记;wasm 后台 timer 不可作状态机收尾,一律用动画值收敛判定;换色致 meta 行增删(40.8px)会引起滚动 clamp,验证脚本每次换色后必须重定位锚点;跨帧签名对比勿用 zip(短列表截断成假 0)。**待验**:桌面原生、Safari/125%、真触摸、官方 viewer 实物(Pro 锁)。 | 部分通过 · `SAVIM□D□` |
| <a id="review-voice-orb"></a>51 [语音球](http://127.0.0.1:4321/kui/components/voice-orb/)<br>[voice-orb](</Users/leon/Documents/Kense ui/KUI/docs/items/voice-orb.md>) | P1 | 球体轮廓、材质/渐变/光晕、各状态颜色和周边控件。 | 空闲/听取/思考/播报/错误/暂停；音量驱动形变、过渡与隐藏时钟。 | 现有文档过于概括；需逐状态记录参数，分清样例与宿主音频。 | 待重审 · `□□□□□□D□` |
| <a id="review-now-playing"></a>52 [正在播放](http://127.0.0.1:4321/kui/components/now-playing/)<br>[now-playing](</Users/leon/Documents/Kense ui/KUI/docs/items/now-playing.md>) | P1 | 封面/标题/时间轴、播放键、音量和队列。 | 播放暂停/拖进度/切曲、加载/错误、时长变化；封面/文本切换、波形与进度同步。 | 不得只切换播放图标；核对音频宿主确认和实际时间推进。 | 待重审 · `□□□□□□D□` |
| <a id="review-voice-recorder"></a>53 [语音录音](http://127.0.0.1:4321/kui/components/voice-recorder/)<br>[voice-recorder](</Users/leon/Documents/Kense ui/KUI/docs/items/voice-recorder.md>) | P1 | 录音/回听/草稿/发送布局、计时、波形和状态键。 | 权限/取消/重录/回听/发送/失败重试；设备释放、草稿保留、波形/计时连续。 | 文档仅描述通用宿主边界，需补具体状态与真实/样例适配验收。 | 待重审 · `□□□□□□D□` |
| <a id="review-image-reveal"></a>54 [图片生成显现](http://127.0.0.1:4321/kui/components/image-reveal/)<br>[image-reveal](</Users/leon/Documents/Kense ui/KUI/docs/items/image-reveal.md>) | P1 | 预览/成图比例、揭示边缘、模糊/颗粒/遮罩和进度标记。 | 开始/取消/失败/重试/旧任务回执、替换图片；软边推进、清晰化及完成收尾。 | 11-10 四图示例和柔化边缘已接入。潜变量噪声、分块解码和完成收尾仍需对照。 | 待重审 · `□□□□□□D□` |

<a id="workflows"></a>

## AI 任务、编辑与宿主交互（21 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-tldr-toggle"></a>55 [摘要切换](http://127.0.0.1:4321/kui/components/tldr-toggle/)<br>[tldr-toggle](</Users/leon/Documents/Kense ui/KUI/docs/items/tldr-toggle.md>) | P1 | 摘要/原文/长度控制、内容卡高度和来源高亮。 | 生成/停止/重试、编辑时的结果处理、引用定位；切换高度、入场和高亮退场。 | 核对整段生成流程，不能把本地样例替换当作真实宿主完成。 | 待重审 · `□□□□□□D□` |
| <a id="review-smart-form-fill"></a>56 [智能表单填写](http://127.0.0.1:4321/kui/components/smart-form-fill/)<br>[smart-form-fill](</Users/leon/Documents/Kense ui/KUI/docs/items/smart-form-fill.md>) | P1 | 原文区、建议字段、置信度标记、审核与保存位置。 | 低置信度审核、编辑冲突、结果排队/接受/拒绝、停止重试；来源高亮和逐字段填入。 | 需区分样例解析与宿主接口；验证已有编辑不会被晚到结果覆盖。 | 待重审 · `□□□□□□D□` |
| <a id="review-enrich-column"></a>57 [数据补全列](http://127.0.0.1:4321/kui/components/enrich-column/)<br>[enrich-column](</Users/leon/Documents/Kense ui/KUI/docs/items/enrich-column.md>) | P1 | 指令编辑器、表格行列、待处理/完成/失败单元格。 | 单行/全部运行、停止/重试、编辑冲突与请求序号；逐格反馈、编辑器高度和焦点。 | 验证乱序/过期结果及重试不重复应用。 | 待重审 · `□□□□□□D□` |
| <a id="review-ticket-router"></a>58 [工单分流](http://127.0.0.1:4321/kui/components/ticket-router/)<br>[ticket-router](</Users/leon/Documents/Kense ui/KUI/docs/items/ticket-router.md>) | P1 | 工单卡、分类/目标队列、低置信度审核区、操作位置。 | 启动/暂停、人工改派、失败重试、宿主确认；工单跨状态移动与退回。 | 指派必须有宿主确认，审查是否只切换了本地标签。 | 312px 审核分页及失败恢复已复测，完整动效和真机待验。 |
| <a id="review-reply-draft"></a>59 [回复草稿](http://127.0.0.1:4321/kui/components/reply-draft/)<br>[reply-draft](</Users/leon/Documents/Kense ui/KUI/docs/items/reply-draft.md>) | P1 | 快捷回复/语气/长度、生成预览、编辑区与插入按钮。 | 停止/重试、生成时编辑、晚到结果确认、插入一次；流式内容和状态切换。 | 检查是否把成段替换当成流式体验；记录宿主/演示边界。 | 待重审 · `□□□□□□D□` |
| <a id="review-ask-your-data"></a>60 [数据问答](http://127.0.0.1:4321/kui/components/ask-your-data/)<br>[ask-your-data](</Users/leon/Documents/Kense ui/KUI/docs/items/ask-your-data.md>) | P1 | 问题框、SQL/结果/图表/表格布局、空/错误状态。 | 提交/停止/重试、复制真实SQL、切换结果视图；图表入场、切换和溢出。 | 核对数据映射及各视图一致，样例查询不可冒充真实执行。 | 三组结果、SQL 复制、方向键、内滚和 312px 失败重试已复测，完整同速动效和真机待验。 |
| <a id="review-pii-redaction"></a>61 [敏感信息脱敏](http://127.0.0.1:4321/kui/components/pii-redaction/)<br>[pii-redaction](</Users/leon/Documents/Kense ui/KUI/docs/items/pii-redaction.md>) | P1 | 选择范围、遮罩/标签、审阅与复制栏。 | 重叠范围、增删/撤销、编辑后偏移、复制前审阅；选择/拖动反馈。 | 固定虚构匹配与本地回复样例；新增回复事件和按请求恢复接口，编辑或撤销审阅使旧回执失效。 | 中文回复恢复及原生取消回归已验证，对齐、操作行和完整动效及平台待验。 |
| <a id="review-dictation-button"></a>62 [听写输入](http://127.0.0.1:4321/kui/components/dictation-button/)<br>[dictation-button](</Users/leon/Documents/Kense ui/KUI/docs/items/dictation-button.md>) | P1 | 麦克风键→听取行、临时文字、状态标记和确认入口。 | 权限/停止/取消、最终文本只插一次、插入位置移动确认、过期回执；宽度与波形变化。 | 需要在实际编辑器内验收，演示无权限不等于真实适配已通过。 | 待重审 · `□□□□□□D□` |
| <a id="review-speech-player"></a>63 [朗读播放器](http://127.0.0.1:4321/kui/components/speech-player/)<br>[speech-player](</Users/leon/Documents/Kense ui/KUI/docs/items/speech-player.md>) | P1 | 播放栏、逐词高亮、进度/速度/暂停控制和换行。 | 启动/暂停/跳转/结束/失败/重试；字词时标、跨行定位、宿主音频同步。 | 必须检查跨行/滚动后的高亮，不只高亮第一句。 | 待重审 · `□□□□□□D□` |
| <a id="review-provider-status"></a>64 [服务商状态](http://127.0.0.1:4321/kui/components/provider-status/)<br>[provider-status](</Users/leon/Documents/Kense ui/KUI/docs/items/provider-status.md>) | P1 | 服务行/状态点、延迟/指标、事故详情、加载和空状态。 | 轮询/数据变化、展开/关闭、打开事故链接、失败恢复；行/详情高度与状态变色。 | 文档写明视觉与触摸待验；需核对具体实际服务状态而非固定绿点。 | 待重审 · `□□□□□□D□` |
| <a id="review-ghost-text"></a>65 [行内文字建议](http://127.0.0.1:4321/kui/components/ghost-text/)<br>[ghost-text](</Users/leon/Documents/Kense ui/KUI/docs/items/ghost-text.md>) | P1 | 真实插入点、灰色建议与正文、单/多行、光标和选择区。 | Tab/分词接受/Esc/编辑、IME、请求取消和晚到结果；流式建议、淡入/退出及位置稳定。 | 文档承认无淡入；精细转场与原生IME/触摸待验。 | 待重审 · `□□□□□□D□` |
| <a id="review-response-feedback"></a>66 [回答反馈](http://127.0.0.1:4321/kui/components/response-feedback/)<br>[response-feedback](</Users/leon/Documents/Kense ui/KUI/docs/items/response-feedback.md>) | P1 | 赞踩键、原因/补充面板、发送/成功/错误布局。 | 选择/反选、提交/重复提交、失败保留草稿、关闭/焦点；面板展开、选中/成功切换。 | 文档明确细节过渡和焦点交接仍待视觉及无障碍验证。 | 待重审 · `□□□□□□D□` |
| <a id="review-image-generation"></a>67 [图片生成](http://127.0.0.1:4321/kui/components/image-generation/)<br>[image-generation](</Users/leon/Documents/Kense ui/KUI/docs/items/image-generation.md>) | P1 | 提示/任务队列、比例控制、像素预览、结果卡/放大/下载布局。 | 开始/取消/失败/重试/放大，逐请求ID与晚回执；预览到成图、放大扫光和进度。 | 文档明确像素马赛克、upscale扫光和完整视觉比对未实现或未验证。 | 待重审 · `□□□□□□D□` |
| <a id="review-meeting-summary"></a>68 [会议纪要](http://127.0.0.1:4321/kui/components/meeting-summary/)<br>[meeting-summary](</Users/leon/Documents/Kense ui/KUI/docs/items/meeting-summary.md>) | P1 | 会议笔记/摘要/行动项、引用、收件目标与发送反馈。 | 生成/停止/重新生成、引用定位、编辑/复制/逐项发送/失败；流式文本、引用hover意图和状态过渡。 | 恢复头像堆叠、摘要／决定／行动项及固定页脚。390px 实测时间戳和单项发送重试。悬停引用和同速时序待验。 | 已返修，待完整对照 · `✓□□□□□✓□` |
| <a id="review-ai-workflow"></a>69 [AI 工作流](http://127.0.0.1:4321/kui/components/ai-workflow/)<br>[ai-workflow](</Users/leon/Documents/Kense ui/KUI/docs/items/ai-workflow.md>) | P1 | 节点/连接器/分支、运行状态、命令菜单和详情高度。 | 执行/停止/重试/分支选择、过期回执；连接器走向、路径交叉淡入、展开和重新定位。 | 统一轨道与分支，补齐节点结果展开、骨架、搜索及键盘添加。实测失败重试和焦点返回。分支交叉淡入及同速时序待验。 | 已返修，待完整对照 · `✓□□□□□✓□` |
| <a id="review-training-run"></a>70 [训练任务](http://127.0.0.1:4321/kui/components/training-run/)<br>[training-run](</Users/leon/Documents/Kense ui/KUI/docs/items/training-run.md>) | P1 | 损失/评估曲线、指标、事件、错误卡与终点状态。 | 开始/暂停/取消二次确认/重试、恢复与宿主回执；平滑曲线、指针scrub、结果卡转场。 | 文档明确曲线直接更新，scrub/结果转场/平台与播报待验。 | 已返修，待完整对照 · `□□□□□□D□` |
| <a id="review-agent-handoff"></a>71 [人工转接](http://127.0.0.1:4321/kui/components/agent-handoff/)<br>[agent-handoff](</Users/leon/Documents/Kense ui/KUI/docs/items/agent-handoff.md>) | P1 | 机器人/人工角色、共享摘要、转交时间线、会话与发送状态。 | 请求/取消/接通/失败/重试、摘要编辑与草稿保留；流式内容、行进点、摘要展开。 | 文档承认流式内容、时间线移动点和完整视觉比对未完成。 | 已返修，待完整对照 · `□□□□□□D□` |
| <a id="review-llm-cost-calculator"></a>72 [模型费用估算](http://127.0.0.1:4321/kui/components/llm-cost-calculator/)<br>[llm-cost-calculator](</Users/leon/Documents/Kense ui/KUI/docs/items/llm-cost-calculator.md>) | P1 | 输入用量/模型/价格、排序/对比条、总价/单位和空/错态。 | 缓存/价格变化、非有限/负数/溢出、加载失败；重排弹簧、总价滚动与读数基线。 | 文档承认重排/总价滚动/加载错误集成及真实端验收待办。 | 待重审 · `□□□□□□D□` |
| <a id="review-agent-cursor"></a>73 [智能体光标](http://127.0.0.1:4321/kui/components/agent-cursor/)<br>[agent-cursor](</Users/leon/Documents/Kense ui/KUI/docs/items/agent-cursor.md>) | P1 | 光标/标签、正文块、选区与编辑前后、插入点定位。 | 开始/停止/接受/撤回、手动编辑/滚动/块移动；逐词写入、选择效果、光标测量和连续跟随。 | 双光标取实际字形坐标，标记错字和新增范围，审核条随动画入场。实测断线重试和撤销。连续动效与真机待验。 | 已返修，待完整对照 · `✓□□□□□✓□` |
| <a id="review-link-unfurl"></a>74 [链接预览](http://127.0.0.1:4321/kui/components/link-unfurl/)<br>[link-unfurl](</Users/leon/Documents/Kense ui/KUI/docs/items/link-unfurl.md>) | P1 | 文本内链接→预览卡的形变、缩略图/元信息、多链接与收起入口。 | 解析/取消/打开/发送、失败保草稿、Alt+Enter、拖回收起；加载→卡片→折叠和晚到响应。 | 文档承认inline morph、多预览、Alt+Enter、拖收起和播报未实现/未验。 | 待重审 · `□□□□□□D□` |
| <a id="review-tone-tuner"></a>75 [语气调整](http://127.0.0.1:4321/kui/components/tone-tuner/)<br>[tone-tuner](</Users/leon/Documents/Kense ui/KUI/docs/items/tone-tuner.md>) | P1 | 语气控制、原文/改写结果、变词高亮、复制/停止/重试布局。 | 快速调节/取消/旧响应、编辑冲突、复制成功失败；句子交叉淡入、替换词运动与结果宽高。 | 文档承认句子交叉淡入与完整参考比对未实现或未验。 | 已返修，待完整对照 · `□□□□□□D□` |

<a id="scenarios"></a>

## AI 状态与过程示意（10 项）

| 编号 / 组件 | 优先级 | 样式与布局需要审查 | 交互与动画需要审查 | 当前证据或已知缺口 | 本轮状态 |
| --- | --- | --- | --- | --- | --- |
| <a id="review-model-training"></a>76 [模型训练示意](http://127.0.0.1:4321/kui/components/model-training/)<br>[model-training](</Users/leon/Documents/Kense ui/KUI/docs/items/model-training.md>) | P1 | 训练阶段、神经网络/节点、损失曲线、参数卡和控制面板。 | 播放/暂停/复位/调参、阶段定位；前向/反向信号、参数更新、曲线时钟和状态同步。 | 文档含未验证声明；必须核对是否为简化示意而非官网体验。 | 待重审 · `□□□□□□D□` |
| <a id="review-multi-region-failover"></a>77 [多区域故障切换](http://127.0.0.1:4321/kui/components/multi-region-failover/)<br>[multi-region-failover](</Users/leon/Documents/Kense ui/KUI/docs/items/multi-region-failover.md>) | P1 | 地图/区域位置、连接弧、状态/延迟/流量读数和故障面板。 | 故障/恢复/区域选择、请求结果；路由重分配、粒子行进、计数和时间顺序。 | 核对宿主数据与演示状态；现有细节/设备验证不完整。 | 待重审 · `□□□□□□D□` |
| <a id="review-data-flow"></a>78 [数据流示意](http://127.0.0.1:4321/kui/components/data-flow/)<br>[data-flow](</Users/leon/Documents/Kense ui/KUI/docs/items/data-flow.md>) | P1 | 源→处理→目标节点、端口/路径/分支、计数及状态色。 | 节点检查、暂停/隐藏、错误/恢复；空心到实心标记、路径运动、数量守恒。 | 文档明确详细路径动效、无障碍和设备行为待验。 | 待重审 · `□□□□□□D□` |
| <a id="review-vector-search"></a>79 [向量检索示意](http://127.0.0.1:4321/kui/components/vector-search/)<br>[vector-search](</Users/leon/Documents/Kense ui/KUI/docs/items/vector-search.md>) | P1 | 2.5D投影、点云/网格、文档卡、查询向量与结果排名。 | 阶段切换/播放暂停、筛选/选中；嵌入→索引→查询→结果的空间移动、相似度展示。 | 文档已承认2.5D几何与细节转场未复现；应列待补，不能因有四阶段按钮通过。 | 待重审 · `□□□□□□D□` |
| <a id="review-ai-spark"></a>80 [智能状态光芒](http://127.0.0.1:4321/kui/components/ai-spark/)<br>[ai-spark](</Users/leon/Documents/Kense ui/KUI/docs/items/ai-spark.md>) | P1 | 八道射线轮廓、尺寸阈值、小图点/大图粒子、Mark/Pill/Avatar。 | 所有状态/自动与受控/速度/暂停/隐藏；旋转收尾、双心跳、错误抖动、文字和色彩混合。 | 文档明确仍无同尺寸60Hz逐帧误差验收；无障碍等价未证明。 | 待重审 · `□□□□□□D□` |
| <a id="review-agent-loop"></a>81 [智能体循环](http://127.0.0.1:4321/kui/components/agent-loop/)<br>[agent-loop](</Users/leon/Documents/Kense ui/KUI/docs/items/agent-loop.md>) | P1 | 环/阶段节点、活动头、工具面板、步骤和时间线。 | 播放/暂停/定位/隐藏、阶段数据更新；头沿路径行进、停在当前位置和阶段联动。 | 11-10 复查官网桌面、手机与单步；重做顺时针圆环、固定的结果和上下文区域，补全播放、工具失败重试及结束。实测单步依次落在收集和工具站，312px 手机预览完整；连续同速录制、真机触摸仍待验。 | 已返修，待完整对照 · `□□□□□□D□` |
| <a id="review-reasoning-tree"></a>82 [推理树](http://127.0.0.1:4321/kui/components/reasoning-tree/)<br>[reasoning-tree](</Users/leon/Documents/Kense ui/KUI/docs/items/reasoning-tree.md>) | P1 | 树层级、分支曲线、节点卡、评分/路径高亮与窄屏大纲。 | 展开/折叠/选择/定位、阶段切换；节点入场、评分增长、路径连接和连续回放。 | 文档明确连续动效与输入验证待办，不能只显示一棵静态树。 | 待重审 · `□□□□□□D□` |
| <a id="review-speculative-decoding"></a>83 [推测解码示意](http://127.0.0.1:4321/kui/components/speculative-decoding/)<br>[speculative-decoding](</Users/leon/Documents/Kense ui/KUI/docs/items/speculative-decoding.md>) | P1 | draft/verify/accepted token布局、空格、拒绝标记、统计。 | 阶段前进/暂停/重播、拒绝与回滚、边界接受率；token跨位置飞行、验证扫过、统计滚动。 | 文档承认跨位置token飞行及滚动统计仍与参考不同。 | 已返修，待完整对照 · `□□□□□□D□` |
| <a id="review-prompt-cache"></a>84 [提示缓存示意](http://127.0.0.1:4321/kui/components/prompt-cache/)<br>[prompt-cache](</Users/leon/Documents/Kense ui/KUI/docs/items/prompt-cache.md>) | P1 | 请求/缓存块、命中/未命中、token/费用/延迟/TTL读数。 | 自动播放/暂停、连续请求、失败/过期/清缓存；缓存飞行、命中动画、倒计时和柱形。 | 文档承认自动播放、cache-flight与倒计时未实现。 | 已返修，待完整对照 · `□□□□□□D□` |
| <a id="review-mixture-of-experts"></a>85 [专家混合路由](http://127.0.0.1:4321/kui/components/mixture-of-experts/)<br>[mixture-of-experts](</Users/leon/Documents/Kense ui/KUI/docs/items/mixture-of-experts.md>) | P1 | token、router、专家/连线、分数/容量/溢出与窄屏布局。 | top-k、容量不足/跳过/并列、选择/暂停/固定；包裹沿曲线移动、分数与选路时序。 | 文档明确实时播报及桌面/Web/视觉/无障碍待验；不得编造未知路由。 | 已返修，待完整对照 · `□□□□□□D□` |

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


### Orbit logos · 2026-11-10

- 公开 Markdown SHA-256 `8d4a91987e81495235189f7e939055b336a2d1e4a1e39ff1a0149e077afedfa7`，公开注册端点 HTTP 404，Pro 端点 HTTP 401。官网实测桌面档轨道 460×460px、内圈半径 115.14px、外圈半径 202px、图标键 52px、中心 78px。390px 手机档轨道 342×342px、图标键 40px、中心 60px，详情面板预留 200px。实际点按 Stripe 打开详情、连接、关闭，再选 Payments 核对 2 项过滤结果。公开文档的默认圈速为 64s / 88s，横排断点为 760px。[官网预览](https://uiarc.dev/components/orbit-logos) · [公开参数](https://uiarc.dev/components/orbit-logos/markdown)
- Rust `OrbitLogo` 将路径及可选着色、图片和自定义标志分成 `with_path`、`with_image`、`with_mark`，并映射类别、说明、初始连接和文档链接。`OrbitLogos` 支持中心内容、默认或受控选择、速度、暂停、名称、全部标签、ID 和布局。选择回调映射为 ID / 索引事件，关闭时为 `None`。连接异步映射为带票据的 `OrbitConnectRequested` 与 `finish_connect`；文档给出宿主接入及默认的本地模拟行为。演示保留 14 个样例标志，中心用 Kense 标志，矩形详情遵循 KUI 8px。
- GPUI 专项 3/3，覆盖跨零点取短路、横排和竖排的内圈避让、方向键筛选导航、回车与 Escape 焦点恢复、失败重试和过期回执。浏览器实际验证 Dev 筛选、Linear 第一次连接失败、重试成功。复拍最终版确认连线避开 GitHub。中文 320px 页面实际画布 288px、页面无横向溢出；横向滚动找到「设计」，打开 Figma，切到下一项 Framer，详情与操作键完整。[亮色外圈停靠](/private/tmp/kui-orbit-logos-ref/kui-final-outer.jpg) · [失败回执](/private/tmp/kui-orbit-logos-ref/kui-failed.jpg) · [连接成功](/private/tmp/kui-orbit-logos-ref/kui-connected.jpg) · [暗色 320px](/private/tmp/kui-orbit-logos-ref/kui-final-320-next.jpg)
- 构建 `34214073f04d6fe67448` 通过格式、Clippy、378 项库测试、WASM 检查和 269 项注册源校验。Story 108 项及 4 项附加测试通过于连线避让修补前。站点校验 231 个双语组件和 947 个目的地。注册包和本地源码分别在独立项目安装，文档例子通过原生及 wasm32。源码 SHA-256 `e8a84edf8595aa32fa1a9374c24aa36fbb0f9484aeff8c7b92baf0e2823bf3c3`，注册包 SHA-256 `d1e51ad95fd452c96af9bd1776480fb4a0e790422fdbcfedf9c8d415f0750998`。[最终构建和安装日志](/private/tmp/kui-orbit-logos-ref/final-gates.log)
- 当前短路径停靠使用共享临界阻尼弹簧。官网的轻微越位、详情切换的出入场和手机逐帧时序还需同速复测。真机触摸、前台 Safari、原生窗口、读屏播报和帧时间待验。

### Ticket tear · 2026-11-10

- 公开 Markdown SHA-256 `ba695c575cbebd542b0e2f8fcb3935ba22734a4704760fe818489a9a43523f26`。公开注册端点 HTTP 404，Pro 注册端点 HTTP 401。以官网公开参数和 DOM、CSS、真实拖动取证。[官网预览](https://uiarc.dev/components/ticket-tear) · [参数与动效](https://uiarc.dev/components/ticket-tear/markdown)
- 官网桌面票面 400 × 210.58px，正文内边距 20px / 24px，标题 36px / 39.6px。手机档票面 310px 宽，票根都为 96px 加 5px 接缝重叠。端部缺口半径 9px，穿孔间距 10px、半径 1.9px。短拖 15px 横向、17px 下移，撕开 32.43px。单轴 20px 横拉增加 15.71px，20px 下拉增加 24.29px。反向拖不回退。实测长拖和 Enter 完成，换票可重置。KUI 使用 8px 外角和自有示例文案。
- API 将正文和票根映射为两个 GPUI 渲染函数，`with_stub_width` 默认 104px，`with_default_torn` 默认 false，`with_label` 提供本地化名称，`with_id` 和 `with_style` 设置稳定身份与布局，`with_paper` 覆盖纸面颜色。`set_torn` 同步宿主状态，`TicketTornChanged(true)` 仅在用户撕下时发送一次。空白项目从打包注册表和本地源码分别安装，文档用法通过原生与 wasm32 编译。
- 原生 GPUI 回归覆盖拖动、反向、短拖保留撕痕、键盘重复、静止淡出及程序重置。浏览器实测英文亮暗、390px 预览、短拖、长拖越出票根、空格和换票。修复了短拖后横向细缝及手机档票面宽度偏差。最终构建 `2496dd47ad34b3d9a004` 在中文 320px 视口下使用 288px 画布，文档无横向溢出，标题完整，长拖后可见换票键。完整构建通过 374 项库测试、108 项 Story 测试和 4 项附加测试，校验 230 个双语组件及 943 个静态目的地。证据保存在 `/private/tmp/kui-ticket-tear-ref/`。官网 243 项中还有 13 项待接入。
- 仍待核对高速甩动阈值、逐帧动效和性能、原生窗口和真机触摸及状态播报。当前纸片会旋转，调用方提供的 GPUI 票根内容只跟随中心位移和透明度。授权源码暂不可核实的弹簧参数由 KUI 实测调校。
- WebKit 探针直接打开英文页已绘制，后台窗口报告 `document.hidden=true` 时，从 Button／Cover flow 切到 Ticket tear 有 URL 已更新而画布停留在上一页的情况。另一个浏览器的封面流 → 票根切换实测正常。Safari 的实际窗口本轮无法读取可访问性树，正常前台路径留待复测。

### 智能体光标 · 2026-11-10

- 官网 Pro 的完整源码需要授权。本轮取证来自公开预览和 Markdown（SHA-256 `897a91c8eff274b1a737ee7a1d9310a3fbe304d05d622a114a9fdb29d3b2ee33`）。官网卡片头高 56px、最大宽 680px，桌面正文边距 32px，标题 20px、正文 15px。默认成功，另有断线入口和重试。[官网预览](https://uiarc.dev/components/agent-cursor) · [公开参数](https://uiarc.dev/components/agent-cursor/markdown)
- 共享控件让错误提示与正文使用同一侧边距，低于 480px 缩到 16px。选区和改动底色按文字范围绘制。预览默认成功，分批写入后在文档内审阅，断线单独触发。失败保留原稿，旧请求票据失效。
- 本地构建 `ea78ed70fcede2ede4e4`。已实测暗色桌面下断线、重试和接受，亮色 390px 手机档下重播和撤销。最终构建 `c29e8d085f8076cf0e0f` 复测 390px 预览，审阅按钮完整可见，缩短中文样例首段避免句号独占一行。保留截图于 `/private/tmp/kui-agent-cursor-ref/`。双路径各自原生、WASM 及文档用法编译通过。
- 待补齐官网双光标、逐行选择及逐段淡入、过渡时序和连续画面对照。桌面原生、Safari、真实触摸、键盘与无障碍尚未逐项验收。

### AI 工作流、会议纪要与智能体光标复查 · 2026-11-10

- AI 工作流的官网公开预览在本次桌面测量中卡宽 522px，标题区约 55px。节点和分支共用一条轨道。KUI 的进站演示依次解析五个宿主回执，入线标记移动 240ms，结果按实测高度展开并淡入。使用 `set_loading` 区分加载与运行。浏览器实测末步失败后只重新执行该步，输入 Linear 后仅剩「更新任务」，回车加入当前分支并把焦点还给添加键。分支内容仍直接切换，应用标记与部分结果样式采用 KUI 实现。[官网预览](https://uiarc.dev/components/ai-workflow) · [官网实测](/private/tmp/kui-trio-review/arc-workflow.png) · [本地完成](/private/tmp/kui-trio-review/kui-workflow-final-done.png) · [本地搜索](/private/tmp/kui-trio-review/kui-workflow-final-filter.png)
- 会议纪要官网实测标题 15px、头部内边距 18/20/16px、头像约 26px。KUI 样例恢复 4 条摘要、3 条决定和 4 个行动项。按 170ms 间隔启动各行、分组显现文字，正文测高后用 240ms 过渡。头部和页脚贯穿生成、错误及无转录状态。实测点击时间戳显示对应引文，在 390px 预览中发送同一项经历等待、失败和重试完成。引用目前为点击展开，手机预览元信息较长时可能在时间范围内换行。[官网预览](https://uiarc.dev/components/meeting-summary) · [本地引用](/private/tmp/kui-trio-review/kui-meeting-quote.png) · [手机档重试完成](/private/tmp/kui-trio-review/kui-meeting-send-ok.png)
- 智能体光标官网实测卡头 56px、正文 15px、侧边距 32px。KUI 两个光标锚定文档排版，活动标签由 `set_activity_for` 更新，`mark_correction_for` 检查 UTF-8 边界。新增范围淡入 140ms，审阅条 200ms 上移 8px。已实测两个光标进入、断线保留原文、重试进入审核及撤销。协作者当前固定跟随首段末尾，官网协作者的连续轨迹仍待核对。[官网预览](https://uiarc.dev/components/agent-cursor) · [官网实测](/private/tmp/kui-trio-review/arc-cursor-current.png) · [本地双光标](/private/tmp/kui-trio-review/kui-cursor-final-playback.png) · [本地重试](/private/tmp/kui-trio-review/kui-cursor-final-retried.png)
- 构建 `c4a960be23b081930253`。三个组件分别从本地生成的注册表安装到独立 crate，文档示例均通过原生和 wasm32 编译。最终 Clippy、站点与注册表检查通过，站点覆盖 231 个双语组件和 947 个静态地址。当前只记录已实测路径，未完成官网逐帧对照、前台 Safari、真机触摸、原生窗口、读屏及 60Hz 帧时间验收。[安装日志](/private/tmp/kui-trio-installs.log)

### Activity rings 与 Sankey flow 专项回归 · 2026-11-09

- [Activity rings 公开 API](https://uiarc.dev/components/activity-rings#api-reference)，Markdown SHA-256 `4f6f93a8ed65c1b3dca6aece2dd40f32121dd9916082e76aada3d35f207002eb`。按同一天的 ID 比较更新前后数值，日期切换与回退日期都不触发达标。用 GPUI 测试窗口检查 608 → 320 → 608px 时的表盘宽度 216 → 184 → 216px，以及方向键将焦点从周二移到周一。专项 3/3。
- [Sankey flow 公开动效](https://uiarc.dev/components/sankey-flow#motion)，Markdown SHA-256 `632ec39d39d7438c116d5f1b0f4e510692e4377ac9fd5657e3ad4ee9c7ac8d33`。GPUI 测试时钟验证聚焦分支时，上游流带按 40% 逐渐填充，清除时保留当下宽度并回落至零。KUI 采用 0.26s 短弹簧。官方文档仅给出短弹簧描述，未公开组件内专用参数。专项 6/6。
- Codex 浏览器实测活动圆环，点周一后按右方向键，周二获焦并显示 79%。网页的 Rust 示例已补齐 `ActivityRings::new` 的 `cx` 参数。
- 浏览器实测桑基图发现流带扭结。修正 `PathBuilder::cubic_bezier_to` 的终点与控制点顺序，命中与粒子使用同一曲线。重新构建后点击「已激活 → 免费」，流带连贯、部分高亮和提示可见。官网同分支实物仍有差异：从中间节点出发时上游叠层是中性色，KUI 仍按各条带来源着色。官网的节点填充按追踪份额、标签垂直居中，提示中含来源与目标比例，这三项仍待修正。[官网预览](https://uiarc.dev/components/sankey-flow)
- KUI Clippy、WASM 发布构建、`verify-preview` 和站点检查通过，站点检查覆盖 193 个组件的双语页面及 795 个静态目标。两项 Pro 源码均未取得。同场景的逐帧时序、Safari 和原生视觉、触摸、独立安装及性能待补证。

### 桑基图追踪及键盘复核 · 2026-11-09

- 公开实物复测 Q3「Activated → Free」。官网稳定态为 3,860，提示显示来源 55%、目标 100%。Organic 节点的总高度为 143px，高亮 51.18px。追踪起于中间列时用中性色（暗色 `oklch(0.8 0 0)`，浅色实测中性色 `oklch(0.33 0 0)`）。选源节点时整条追踪使用该节点系列色。[官网预览](https://uiarc.dev/components/sankey-flow)
- KUI 按节点、流带的实际追踪份额绘制叠层，清除时从当前宽度或高度回退。数值格式可由宿主提供，链接提示给出源和目标比例，节点提示给出占总量比例。普通鼠标悬停跟随指针，触摸点选的固定及再次点按清除还需真机核对。
- 官网页宽测得列名 12px / 20px，横排节点标签组高 32.2px，标签在节点右侧 7px 并居中。移动预览图幅 310×480px，最后一列起点 y=430px，标签 y=446px。KUI 已据此校正列名、节点标签及直立布局。颜色仍经 KUI 主题 Token 换算，外框圆角遵循 KUI 的 8px 约定。
- GPUI 测试窗口中，桑基图通过 Tab 进入、同列移动、跨列选最近节点、Home/End、Escape 六个操作。活动圆环同时验证 Tab 进入已选日期，再用方向键移焦。Sankey flow 7/7，Activity rings 3/3。新增高度、空态、数值格式和粒子受控 API 已写入组件文档。
- 本地英文版实拍同一 Q3 分支，确认节点部分填充、中性色、完整三行提示与 Escape 清除。提示按实排尺寸定位，已消除 `100% of Free` 被截断。320 CSS px 下最后一列标签完整，但 Pro 和 Team 的数值间距过小，窄屏视觉阶段仍未通过。
- 最终版通过 Clippy、WASM 和站点构建、`verify-preview`。分别从独立目录安装并编译文档用例，Sankey flow 的 JSON 与本地源码两条路径、Activity rings / Split bar / Waffle chart 的 JSON 路径均通过原生与 `wasm32-unknown-unknown`。华夫图修正动态数字依赖及旧构造参数，华夫图和占比条均补齐共用颜色模块。
- 公开预览和文档是此次 Pro 取证范围。完整静态像素对照、同速中间帧、Safari/原生/真触屏和性能仍需补验，组件未标为通过。

### 桑基图窄屏标签专项 · 2026-11-10

- 官网手机档 iframe 的布局宽为 390px，图幅 310×480px。标签左沿对准流带起点，下移 16px，宽度上限为流带宽度加 8px。14px 段间距给标签留下 6px。Q2 的 Team 标签上限实测 24.512px，文字需要约 31px，使用省略号。完整数值和名称仍可通过图表检查及数据表取得。[官网预览](https://uiarc.dev/components/sankey-flow) · [公开参数](https://uiarc.dev/components/sankey-flow/markdown) · [手机档实拍](/private/tmp/kui-228-release/sankey-arc-mobile.jpg)
- KUI 在窄屏缩小横向留白，按图幅限制标签。末项需要避让右缘时，按实际文字宽度向左使用剩余空间，与相邻文字保持 6px。320px 中文文档实际画布宽 288px。Q3 的「专业版 2,310」「团队版 840」完整显示，点击 Q2 后「团… 530」和相邻读数分开；英文亮色 390px 手机档的 Q3 标签与读数完整。特长文案仍可省略，完整名称及数值保留在提示和数据表。[320px 修复前](/private/tmp/kui-228-release/sankey-kui-mobile-before.jpg) · [修复后 Q3](/private/tmp/kui-228-release/sankey-kui-mobile-after.jpg) · [修复后 Q2](/private/tmp/kui-228-release/sankey-kui-mobile-q2.jpg) · [390px 亮色](/private/tmp/kui-228-release/sankey-kui-mobile-390.jpg)
- 桑基图专项 8/8。新增用例验证 238px、310px 图幅的五个过渡进度点，末项完整且相邻文字间距至少 6px。格式、Clippy、WASM 检查、注册表校验及站点构建通过。最新站点 `f302e82a16131309f299`，校验 230 项双语预览和 943 个目的地。独立项目从注册包及本地源码安装，文档例子均通过原生和 wasm32 编译。源码 SHA-256 `b4914e860d5fac8047060fa347f6a18bfc13a4eb8fe8b587c5b3dfca5e711aab`，注册包 SHA-256 `980e1463e0128da274f38865bf1cf9b7cb0a726006eb97c4a67f001066f94d40`。[安装日志](/private/tmp/kui-228-release/sankey-mobile-install.log)
- 此次关闭窄屏标签贴边问题。桌面静态全状态、同速连续画面、真机输入、读屏及帧时间仍需验收。

### 三个独立主题变体 · 2026-11-09

- 核对公开注册包，Eclipse、Split、Rise 与 Reveal 的 TSX、CSS 文件分别相同（SHA-256 前缀 `ab77cf084065`、`271d73fdd37b`）。三个新变体沿用同一 Rust 控件和独立安装 ID。官网主预览均为 36px 高的文字按钮。Rise 文档还展示 38px 图标键，KUI 保留相应的 `with_icon_only` 选项。官网实现将页面动画交给宿主。 [Eclipse 源码](https://uiarc.dev/r/theme-switch-eclipse.json) · [Split 源码](https://uiarc.dev/r/theme-switch-split.json) · [Rise 源码](https://uiarc.dev/r/theme-switch-rise.json)
- 三个变体的键盘和鼠标专项测试均核对了目标主题、模式及触发边界。Web 实测可见日蚀形遮罩、Split 中缝和 Rise 纵向位移，切换组件复用同一画布。快速双击后回到起始主题，遮罩清理完成。快点的间隔和闪白需要连续帧证明。
- 三个 ID 的注册 JSON 独立安装与文档示例均通过原生和 WASM 编译。Rise 另通过本地源码安装。KUI 宿主的 430/390/360ms 是当前实现，官网宿主的逐帧时长、可逆轨迹、Safari 和真触摸待测。

### 三个主题变体的图标对齐返修 · 2026-11-10

- 复核三份官方公开源码，同一个 `iconWrap` 在深色时按变体应用位移：Eclipse 左 2px、Split 右 1px、Rise 上 2px。按用户最新要求，KUI 刻意统一图标中心，不复刻这些位移。三种整页遮罩及宿主时长保持原设定。
- 浏览器逐个实测深 → 浅 → 深。三张 628 × 220px 深色稳定预览 PNG 的 SHA-256 一致（前缀 `ad3fc6ba0ae2`），包含相同位置的月亮与按钮外框。确认最终状态，未据此推断中间帧与官网相同。[日蚀](/private/tmp/kui-speculative-ref/local-theme-switch-eclipse.png) · [中缝](/private/tmp/kui-speculative-ref/local-theme-switch-split.png) · [升起](/private/tmp/kui-speculative-ref/local-theme-switch-rise.png)

### 已接入条目安装复验 · 2026-11-09

- 对 198 项清单中排除原 108 项后的 90 个 ID，各建一个临时 crate，从生成的 JSON 安装，编译该条目 Markdown 的首段 Rust 示例。原生与 WASM 均为 90/90，通过的清单是当时官网覆盖表已接入的全部 90 项。注册目录 SHA-256 `9d9a79f1872e58d320e6e9dee45de50c187e788079fe852e87bebbd27f986b37`。
- 修正 Funnel chart 的上下文参数、Race bar chart 的时间帧数据和 Slope chart 已取消的颜色参数。对应中英文网页代码页同步示例。AiSpark 与 MorphLoader 可使用任意变量名，脚本移除了硬编码的 `control` 变量假设。
- 该复验覆盖安装依赖和首段 Rust 示例。其余 API、不同运行状态及同场景视觉、动效仍按每项独立检查。

### Glass card · 2026-11-09

- 公开 Markdown SHA-256 `43d79b75fbe9d9f3dea50f4a283bf99898168c631840ec4af109d64822aa22a6`。官网 Code 面板要求 Pro 权限，公开注册包返回 404。参数核对覆盖图源、替代文字、标题、可选副标题/右侧/详情、受控与非受控展开、事件及 `tilt=6`。样式和宽度通过 GPUI builder 接入。[官网预览](https://uiarc.dev/components/glass-card) · [公开文档](https://uiarc.dev/components/glass-card/markdown)
- 官网桌面实测 360×450px，玻璃面板内缩 12px、收起约 73px、展开约 240px，背景滤镜为 22px blur 和 1.7 饱和度，图源从 1.001 倍缩放到 1.05 倍。暗色为深灰、亮色为白色，两者 alpha 均为 0.62。标题 16px/500，副标题和简介 14px，两个详情操作各高 44px。官方卡片圆角 34px、面板圆角 22px。[官网收起](</private/tmp/kui-glass-reference-closed.jpg>) · [官网展开](</private/tmp/kui-glass-reference-open.jpg>)
- 共享 Rust 实现使用 8px KUI 圆角、原创示例图、测量详情高度、按图像 ID 缓存模糊贴图、统一的亮暗色、底边固定的面板弹簧及位置可逆的指针高光。专项测试验证 280×350 容器、详情独立事件、Esc 焦点返回、受控事件与回写分离、65ms 中途反向保持高度连续、减少动效即时切换几何。
- 最终 Web 版本 `8adf50fad1a90869a831`。中英文预览实测展开、关注和预约各自更新、Esc 收起，以及展开时切主题保持状态。英文介绍按本地字体实调为 13.5px，恢复官网的两行换行和约 241px 展开高度。320px 中文页内嵌宽 288px、卡片 280×350px，两个操作均完整显示。[暗色英文展开](/private/tmp/kui-glass-local-open.jpg) · [亮色英文展开](/private/tmp/kui-glass-local-light.jpg) · [浅色中文 320px](/private/tmp/kui-glass-local-320.jpg)
- 独立项目分别从生成注册 JSON 和本地源码安装，文档 Rust 示例通过原生及 WASM 编译。注册包 SHA-256 `5297668f0535321450395013e4446dd2c300aa660dad8ed95e224039f6b0bb05`，控件源码 SHA-256 `6fb3d7be8223d97f3a724484aeb43a1fc86efa930024828a6f79db0f2142c81e`。格式、Clippy、故事测试、WASM、注册表与站点双语生成通过。
- 当前差异包括 8px 圆角、原创图源、GPUI 的压缩位移倾斜与官网的透视旋转，以及缓存模糊贴图与网页实时背景滤镜。展开淡入有上移，详情关闭时随即移除。减少透明度由宿主设置。官网同速连续帧、Safari 与原生实拍、真机触摸、读屏和帧时间待测。

### Social proof toast · 2026-11-10

- 公开 Markdown SHA-256 `e171ed89b9fabdb6504f95bbabbc0f70d30bc20e1702c29a74be2915c4f527bc`。官网 Code 页提示需 Pro，公开注册入口返回 404。已核对事件字段、最旧在前、时间与核验标识、4000ms 间隔、800ms 初始延迟、三张默认可见、默认预填充 0、四角与内联定位、展开、循环、会话上限、外部暂停及三种回调。[官网预览](https://uiarc.dev/components/social-proof-toast) · [公开参数](https://uiarc.dev/components/social-proof-toast/markdown) · [KUI API](../items/social-proof-toast.md)
- 官网 1540px 暗色实测 744×300px 示例，堆叠布局 360×84px，单卡 360×64px，头像 36px、关闭控件 24px。后两层各向上 10px、缩放为 95% 和 90%，扇出后每层移动 72px，布局仍预留 84px。聚焦期间 4.4 秒内前项保持不变。Escape 关闭后焦点移到下一张，原来退到后面的记录补入第三张。390px 视口内联卡宽 272px。[收拢](</private/tmp/kui-social-ref/arc-idle.png>) · [扇出](</private/tmp/kui-social-ref/arc-expanded.png>) · [关闭](</private/tmp/kui-social-ref/arc-dismissed.png>) · [窄屏](</private/tmp/kui-social-ref/arc-mobile.png>)
- 本地暗色 `1d9d917ddae2fb903530` 复查悬停展开、离开后恢复轮播、Escape 移除当前项和补入旧记录。GPUI 测试验证暂停保留剩余 500ms、恢复继续计时，以及关闭后移焦和减少动效。中文浅色 `f4eac65d680dd6724cb2` 在 320px 视口无横向溢出，元信息在同一行截断，核验标识仍可见。[展开](</private/tmp/kui-social-ref/final-expanded-dark.png>) · [恢复](</private/tmp/kui-social-ref/final-resumed-dark.png>) · [关闭](</private/tmp/kui-social-ref/final-dismissed-dark.png>) · [320px](</private/tmp/kui-social-ref/latest-narrow-zh.png>)
- 两条独立安装路径与文档用例通过原生和 WASM 编译。源码 SHA-256 `3a75bb77cbd897649953edb6a535f34e2298bfc407b2a3690b1f177455064dc7`，注册包 SHA-256 `02a2417e23382063dedcdc45b70eb9966fcbe14205445d51c8ae7ece4be297ad`。
- 当前采用 KUI 8px 卡片圆角及原创样例。两端同速运动、真实 Safari、原生 GPU、读屏、触摸和帧耗时仍待验。

### Photo grid · 2026-11-10

- 公开 Markdown SHA-256 `2227257b05309180c8a620a8c61886e315222d4a980c402f3ca83dd8de121b71`。公开注册入口 404，Pro 注册入口 401。核对相册标题、照片稳定 ID、替代文字、完整图源和缩略图、原始尺寸、可选详情、默认收藏 ID、默认 3 列及 `[5, 3, 2]` 三档密度。官网用法里的初始 4 列在 KUI 可独立于三档使用。缩放、收藏、查看状态映射为 `PhotoGridEvent`。[官网预览](https://uiarc.dev/components/photo-grid) · [公开参数](https://uiarc.dev/components/photo-grid/markdown) · [KUI API](../items/photo-grid.md)
- 官网 1280px 实测面板 521.6 × 380px，网格 3px 间距，三列格子约 169.2px，图片圆角 6px。首张照片查看区原图 153.4 × 230px，操作键 40px，胶片条 64px。390px 视口面板宽 304px，操作键 36px、胶片条 60px，隐藏左右导航。实际操作完成三列切五列、打开、收藏和下一张，胶片条显示 2/16，Escape 焦点返回第二张。[官网网格](/private/tmp/kui-photo-ref/arc-grid.png) · [查看器](/private/tmp/kui-photo-ref/arc-open.png) · [窄屏](/private/tmp/kui-photo-ref/arc-mobile-open.png)
- KUI 采用 8px 外框和本地公有领域相册素材。密度变更从当前照片边界重排，查看器从照片单元展开。GPUI 回归涵盖重排前后位置、焦点回到当前照片、收藏事件、鼠标横向滑动及下拉关闭。官网未公开动效常量，当前参数按实测建立视觉目标，实际值在 Rust 源码维护。
- 首轮浏览器 `80655c8fb74fe51409fc` 从三列切到五列，内滚到末张时外层 `scrollY=0`。收藏第 16 张后相册计数 2→3，胶片条向右拖 82px 切到第 13 张，下拖 121px 退回网格。中文 320px 视口的横拖从第 1 张到第 2 张，按钮和胶片条均保持可见。[五列内滚](/private/tmp/kui-photo-ref/kui-five-scrolled-en.png) · [胶片条拖动](/private/tmp/kui-photo-ref/kui-strip-scrub-en.png) · [窄屏横拖](/private/tmp/kui-photo-ref/kui-320-viewer.png)
- 最终 Web `32905a3ef66b899ff8b7` 复测中文亮暗与窄屏。320px 无横向溢出，预览宽 288px，关闭键可点。1280px 暗色按 Right 切到第 2 张，Escape 返回。最终新增的 GPUI 捏合测试从独立初始 4 列开始，移动时连续缩放，松手切到 3 列且首帧延续当前宽度。格式、Clippy、367 项组件测试、108 项 Story 单测及 4 项集成测试、WASM、双语网站 935 个静态地址通过。注册 JSON 与本地源码安装分别通过原生和 WASM 文档用例。共享源码 SHA-256 `5d5588be3ed83d8c410948ecb126d64dfc6542018ee30bf3e0323b33495e1027`，注册包 SHA-256 `d81b0838dd45c0ea47cb7af503a1c82c684e10ac2ac92e5fbc8298ea4b5ffb59`。[320px 查看器](/private/tmp/kui-photo-ref/kui-final-320-open.png) · [暗色查看器](/private/tmp/kui-photo-ref/kui-final-dark-viewer.png) · [中文交付画面](/private/tmp/kui-photo-ref/kui-delivery-zh.png)
- 差异仍包括相册素材、字体、8px 外框及不透明面板，宽屏无悬停设备目前仍显示左右键，收藏计数采用即时文本。官网同速逐帧、Safari、原生 GPU、真机捏合与滑动、减少透明度、读屏和帧耗时待验。

### Card to article · 2026-11-10

- 公开 Markdown SHA-256 `f88d680240d70e5758463a98f8851222da92fe6e098c10e85b31ba4832109430`。公开注册入口 404，Pro 注册入口 401。核对稳定 ID、图片及替代文字、标题、正文、可选标签与元信息、受控与非受控选择、默认关闭、4:3 比例与关闭键名称。打开与关闭回调映射为 Rust 事件，宿主可选接入网页滚动锁。[官网预览](https://uiarc.dev/components/card-to-article) · [公开参数](https://uiarc.dev/components/card-to-article/markdown) · [KUI API](../items/card-to-article.md)
- 官网 1280px 实测网格宽 521.6px、每张卡片 252.8 × 189.6px、间距 16px。展开后的文章 680 × 672px，封面 680 × 510px，关闭键 36px、右上各内缩 14px，阅读线 3px。390px 视口改为单列，卡片 304 × 228px、间距 12px，文章 374 × 996px。实测手机横向内缩 8px，公开文档写 12px，KUI 采用实测横向值与文档的 12px 纵向值。实测短拖回弹、长拖关闭、Tab 留在文章、Escape 回到原卡片。[官网网格](/private/tmp/kui-article-ref/arc-grid.png) · [官网展开](/private/tmp/kui-article-ref/arc-open.png) · [窄屏展开](/private/tmp/kui-article-ref/arc-narrow-open.png)
- 共享 GPUI 实现按同一宽度计算封面、标题、标签及元信息，并在关闭时读取重排后的卡位。专项测试覆盖受控状态等待宿主回写、拖动与甩动判定、Enter 打开、Tab、Page Down 和缩窗后 Escape 还焦点。KUI 采用 8px 表面圆角及 CC0 图源。内嵌文章和遮罩使用文档预览画布的可见区域，独立 GPUI 控件使用全窗口。官网弹簧参数缺少公开值，当前使用 KUI 自有时长。
- 浏览器复测滚轮阅读时外层 `scrollY` 保持原位，短拖 17px 留在文章，长拖 156px 关闭。切到代码页解除外层滚动锁，回到预览后恢复文章。增加 GPUI 回归验证拖动过程中面板位移、使用释放坐标判定距离、关闭键不会启动封面拖动。[英文打开](/private/tmp/kui-article-ref/kui-final-open-en.png) · [英文内滚动](/private/tmp/kui-article-ref/kui-final-reading-en.png)
- 最终 Web `70f4bcdac336cf174b8a`。中文 320px 视口、288px 预览、正文 `scrollY=589` 时从第四张卡片打开。面板避开 57px 固定页眉，关闭键完整可点，关闭后正文位置不变。暗色中文以 Page Down 翻页，进度线随正文更新，Escape 恢复原卡片。格式、Clippy、363 项组件测试、108 项 Story 单测及 4 项集成测试、WASM 与双语网站 931 个静态地址检查通过。注册 JSON 和本地源码两条安装路径，文档用法均通过原生与 WASM 编译。控件 SHA-256 `b337ab182b5a41e7cb0dbf108256a75364975629ed21edb74e2f60aa9872a441`，注册包 SHA-256 `93a82e71a90863d4174bc2c51c732172931c7602c34207554eeb1b92c716cc24`。[320px 打开](/private/tmp/kui-article-ref/kui-delivery-320-settled.png) · [暗色翻页](/private/tmp/kui-article-ref/kui-delivery-dark-reading.png) · [中文网格](/private/tmp/kui-article-ref/kui-delivery-zh.png)
- 官网同速逐帧、Safari、原生 GPU 实拍、真机触摸、读屏与帧耗时待验。滚动经过封面时 GPUI 使用矩形内容裁切，面板顶边的遮罩细节仍有差异。

### Upgrade prompt · 2026-11-10

- 公开 Markdown SHA-256 `1e9bb66efd13e30512fd625a6320b07d593cc962f543f952ac558a765bf5ab95`。公开注册入口 404，Pro 注册入口 401。核对了用量 50／50、Nov 1 恢复、默认年付、月付 20 USD、年付折算每月 16 USD、折扣 20%、年付合计 192 USD、新额度 2,000、每次确认保留提交时选择、失败后重试、稍后返回及 1.1 秒模拟成功。[官网实物](https://uiarc.dev/components/upgrade-prompt) · [公开参数](https://uiarc.dev/components/upgrade-prompt/markdown) · [KUI API](../items/upgrade-prompt.md)
- 官网 1280px 暗色测量为 440×414.8px，外边内缩 25px（含边框），标题 18px／500，正文 14px，用量条 6px，底部按钮高 44px。月付、年付、处理中、成功及稍后提醒保持同一外框；年付显示每月 16 USD，模拟账单显示 192 USD。公开时长为用量 500ms、滚动数字 280ms、文本和面板 100ms 退出及 160ms 进入。[月付](</private/tmp/kui-upgrade-ref/arc-monthly-full.png>) · [年付](</private/tmp/kui-upgrade-ref/arc-yearly-full.png>) · [稍后](</private/tmp/kui-upgrade-ref/arc-reminder-full.png>) · [成功](</private/tmp/kui-upgrade-ref/arc-success-full.png>)
- GPUI 回归验证桌面 440×415px，320px 窄屏中错误与成功保持相同卡片尺寸，月付提交金额 2,000、年付 19,200（货币百分之一），连续请求只启动一次，失败保留选择并可以重试，成功后焦点回到主键。示例不接入实际支付。宿主可配置受控周期，事件与实际账单任务分离。
- Web 构建 `348b039ff3df01f4734b` 实测月付、Right 返回年付、提醒和重开、提交中、年付成功及继续回到报价。320px 中文预览宽 288px，页面无横向滚动，计费切换移到价格下方，两个底部按钮完整。[滚动中间帧](</private/tmp/kui-upgrade-ref/kui-final-price-mid.png>) · [键盘](</private/tmp/kui-upgrade-ref/kui-final-keyboard-yearly.png>) · [提交中](</private/tmp/kui-upgrade-ref/kui-final-pending-dark.png>) · [英文成功](</private/tmp/kui-upgrade-ref/kui-final-success-dark.png>) · [320px 报价](</private/tmp/kui-upgrade-ref/kui-final-320.png>)
- 最终 Web `9598cf4ab6d0bd26d4bb`，缩短中文成功说明后复测 320px 报价及完成态，文案完整，账单显示 192 USD 模拟金额。格式、Clippy、358 项组件和 108 项 Story 单测、4 项集成测试、原生／WASM 编译、源码哈希校验与 927 个静态地址通过。两条独立安装路径和文档用例在原生及 WASM 上通过。控件源码 SHA-256 `bb14bdd63b1e97e839f998da5780421b2b04f12bc2d671b44194772b330f636f`，注册包 SHA-256 `22d25ed8c558a6e5c901fc7787d456d77fd322fdd9e5be1d431e5e0c40b2691b`。[中文桌面](</private/tmp/kui-upgrade-ref/kui-delivery-zh.png>) · [320px 成功](</private/tmp/kui-upgrade-ref/kui-delivery-320-success.png>)
- 视觉差异包括 KUI 8px 外框及操作键、字体、可配置的模型文案和标记，以及 320px 时文档画布的可用宽度。尚缺同速逐帧、Safari、原生 GPU、真机触摸、读屏和帧耗时证据。错误与重试本轮使用 GPUI 测试宿主验证，真实账单由接入方负责。

### Referral card · 2026-11-10

- 公开 Markdown SHA-256 `ac708b050967639213b563b9c621c5d114691005f80810eae71f5b13bdfd2c7b`。官网 Code 页标明 Pro 权限，公开注册入口返回 404。已核对链接、20 单位奖励、USD、标题和说明、邀请记录与阶段、5 个付费邀请里程碑、12 段上限、分享文案、10 个地址上限、模拟开关及四类回调。Rust 的邀请取消信号由宿主检查，组件取消或销毁时停止等待结果。[官网预览](https://uiarc.dev/components/referral-card) · [公开参数](https://uiarc.dev/components/referral-card/markdown) · [KUI API](../items/referral-card.md)
- 官网 1540px 暗色预览实测卡宽 560px、高约 605px、内边距 24px、输入与分享键高 44px、名单视口高 182px。390px 视口卡宽约 304px，分享键移到独立一行，名单高度不变。重播取样显示 3／5 → 4／5 → 5／5，奖励 $60 → $80 → $100，模拟新邀请先进入名单顶部，随后依次注册、赚取。[桌面已解锁](</private/tmp/kui-referral-ref/arc-unlocked.png>) · [390px 官网](</private/tmp/kui-referral-ref/arc-mobile.png>) · [重播取样](</private/tmp/kui-referral-ref/arc-replay.json>) · [模拟发送取样](</private/tmp/kui-referral-ref/arc-send.json>)
- 首轮本地浏览器在 1540px 暗色版复测重播的 3／5 → 4／5 → 5／5。输入不完整邮件后显示错误，粘贴两条地址及其中一条的大写重复项，实际只生成两条。连续按回车保持一个等待过程，之后两个样例进入列表，第一条单独推进到赚取。四次发送取样外层预览高度均为 697px。名单滚轮到末项，正文 scrollY 保持 0。[重播](</private/tmp/kui-referral-ref/kui-first-replay-4850.png>) · [无效地址](</private/tmp/kui-referral-ref/kui-first-invalid.png>) · [发送中](</private/tmp/kui-referral-ref/kui-first-sending.png>) · [名单新增](</private/tmp/kui-referral-ref/kui-first-send-1750.png>) · [名单末项](</private/tmp/kui-referral-ref/kui-first-list-bottom.png>)
- 最终 Web `a4dfdd9c48d908929333`。修复窄屏里程碑文字截断和独立安装缺少的 `flyout` 依赖。320px 中文画面预览宽 288px，无横向溢出，达成前文案和解锁后的说明均可完整阅读。复制后粘贴到本地搜索框，确认是完整邀请链接。GPUI 专项测试覆盖重复提交、发出后输入只读但保留焦点、失败保留标签、取消信号与迟到结果、里程碑回调只发一次、182px 名单高与分享 URL 编码。[320px 初始](</private/tmp/kui-referral-ref/kui-final-narrow-initial.png>) · [320px 达成](</private/tmp/kui-referral-ref/kui-final-narrow-unlocked.png>) · [复制反馈](</private/tmp/kui-referral-ref/kui-final-copied.png>)
- 格式、Clippy、350 项组件测试、108 项 Story 单测与 4 项集成测试、原生／WASM 编译、预览源码校验和 919 个静态地址检查通过。注册 JSON 与本地源码两条安装路径的 Rust 用法均通过原生和 WASM 编译。源码 SHA-256 `02e0e147cce431d68bb16eb78e77de6e8c77a7598c95f00c6ea08d0ac71d5dfb`，注册包 SHA-256 `b63381faeeec28d56ac00167c141db6bdc2f4994c5cf99b1ab561cea21ef9f23`。
- 当前采用 KUI 的 8px 矩形圆角、字体及原创样例。X 和邮件把分享文案编码到 URL，LinkedIn 使用带链接的 share-offsite 地址。预览宿主报告可见状态，GPUI 内部以卡片面积 40% 启动样例；外层浏览器滚动的精确触发点待验。真实分享服务、官网同速逐帧、Safari、桌面 GPU、真机触摸、读屏和帧耗时继续待验。

### Share sheet · 2026-11-10

- 公开 Markdown SHA-256 `cf543cc787c329dbdc3cc2246d96f4c191fad72950e304e6e1927da75739b971`。官网 Code 页提示源码需 Pro，公开注册 URL 返回 404。已核对必需的标题、链接、人员与权限，空渠道默认值、默认取第一权限、默认 `Share`、默认 End、520px 手机阈值、发送／渠道／复制回调及自定义宽度与样式。[官网预览](https://uiarc.dev/components/share-sheet) · [公开参数](https://uiarc.dev/components/share-sheet/markdown) · [KUI API](../items/share-sheet.md)
- 官网暗色桌面实测 92×36px 触发键、384px 外壳、368×433px 面板内容区、34px 复制键、52px 权限行、40px 渠道键、40px 头像和 44px 发送键。确认面板约 304×192px。选择权限时列表在同一面板向下扩展。发送失败保留收件人，成功显示已送达及 Undo／Done，打开后焦点在 Copy。390px 手机预览面板呈现底部遮罩与拖动条。[桌面打开](</private/tmp/kui-share-ref/arc-open.png>) · [权限列表](</private/tmp/kui-share-ref/arc-access.png>) · [等待](</private/tmp/kui-share-ref/arc-pending.png>) · [失败](</private/tmp/kui-share-ref/arc-failed.png>) · [成功](</private/tmp/kui-share-ref/arc-sent-settled.png>) · [手机](</private/tmp/kui-share-ref/arc-mobile-settled.png>)
- 最终 Web `6a9c57dfd08094366128`。1540px 暗色版采集打开后约 41／120／232／502ms 的窗口截图。首轮发现文字在展开中被清晰裁切，调整为外壳先伸展、内容随可见面积显现。发送模拟渠道从等待到已发布，选人时头像飞到收件人栏。中文界面实测失败保留原收件人、切换模拟结果后原位重试、成功焦点到完成、撤销返回面板。点击复制后粘贴至本地搜索输入，确认完整 URL 一致。[120ms](</private/tmp/kui-share-ref/kui-final-open-120.png>) · [502ms](</private/tmp/kui-share-ref/kui-final-open-500.png>) · [头像移动](</private/tmp/kui-share-ref/kui-final-flight.png>) · [中文失败](</private/tmp/kui-share-ref/kui-final-zh-failed.png>) · [中文成功](</private/tmp/kui-share-ref/kui-final-zh-sent.png>)
- 320px 中文版内嵌宽 288px，页面无横向溢出。权限列表展开后滚轮到发送键，外层正文 scrollY 保持 0。实际拖动条向下 106px 释放后回到分享键。桌面约 65ms 打开后按 Escape，截图确认已返回关闭态。GPUI 回归覆盖权限循环选择和逐级 Escape、重复提交拦截、失败重试、原生剪贴板写入与权限回调、短拖回弹和长拖关闭。[手机打开](</private/tmp/kui-share-ref/kui-final-mobile-open.png>) · [滚动到底](</private/tmp/kui-share-ref/kui-final-mobile-scroll.png>) · [拖动关闭](</private/tmp/kui-share-ref/kui-final-mobile-drag.png>) · [快速反切](</private/tmp/kui-share-ref/kui-final-reversal.png>)
- 格式、Clippy、347 项组件测试、108 项 Story 单测和 4 项集成测试、WASM 构建、预览源码校验和 915 个静态地址通过。空白项目从注册 JSON 和本地源码两条路径安装，文档示例均通过原生与 WASM 编译。源码 SHA-256 `a7e8542b8030216ca188542aa8d7ce4d1b9171567fb4537d63e82403f06516df`，注册包 SHA-256 `57c40fe8f9238c0b0d3de337a4eac54b5ee88fa3a91e01b8dc7cde827da3abff`。
- 差异包括 KUI 8px 面板、示例人物、渠道图标和字体。面板以淡入和少量位移替代官网的模糊及缩放。手机遮罩覆盖承载 GPUI 的预览框，独立应用覆盖 GPUI 窗口。示例发送由模拟服务提供，撤销回到选人态。真实服务撤销、读屏、Safari、桌面 GPU 实拍、真机触摸、精确同速对照和帧耗时继续待验。

### Skeleton morph · 2026-11-10

- 官网公开 Markdown SHA-256 `26e16c5a71f91fbc398cab3a20e5aef18a5532ae80cae8cf1eb44b0f28511e3d`。公开安装入口返回 404。核对了必需的加载状态和内容、默认 40ms 错峰、`Loading` 标签、区域元素和样式，以及占位宽、高、圆角、行数和默认 20px 行高。KUI 使用 `SkeletonMorph`、`MorphBlock` 与 `MorphLayout`，由宿主加载数据并传入状态和样式。[官网预览](https://uiarc.dev/components/skeleton-morph) · [公开参数](https://uiarc.dev/components/skeleton-morph/markdown) · [KUI API](../items/skeleton-morph.md)
- 官网英文版 1540px 视口，两个卡片均宽 312px、间隔 16px，完成态高 222／214px。资料头像 52px，操作键 90×36px，列表头像 36px，重载键 98×36px。390px 下两卡宽 272px、间隔 16px，垂直排列。重载约 1.4 秒后逐块显现，关注状态保留。[完成态](</private/tmp/kui-skeleton-ref/arc-ready.png>) · [加载态](</private/tmp/kui-skeleton-ref/arc-loading.png>) · [宽版测量](</private/tmp/kui-skeleton-ref/arc-layout-ready.json>) · [窄版测量](</private/tmp/kui-skeleton-ref/arc-layout-mobile.json>)
- 本地预览 `7653d87b23ea580ce7fd`。首轮浏览器发现完成态点击关注会暂时拉高两张卡，修正为只在显现期约束高度，完成后恢复自然布局。新版点击关注时卡边和重载键位置稳定。再次重载，约 76ms 的取样仅见骨架、1536ms 可见内容逐块显现、2303ms 已稳定且保留关注。空格触发重载，暗色完成态及 320px 中文窄屏已复测。窄屏画布宽 288px，两张卡和重载键均可见，页面无横向溢出。[亮色完成](</private/tmp/kui-skeleton-ref/final-ready-light.png>) · [关注](</private/tmp/kui-skeleton-ref/final-follow.png>) · [重载骨架](</private/tmp/kui-skeleton-ref/final-reload-75.png>) · [重载中间帧](</private/tmp/kui-skeleton-ref/final-reload-1530.png>) · [重载完成](</private/tmp/kui-skeleton-ref/final-reload-2300.png>) · [暗色](</private/tmp/kui-skeleton-ref/final-ready-dark.png>) · [320px 中文](</private/tmp/kui-skeleton-ref/final-320-zh.png>)
- 格式、Clippy、343 项组件测试、108 项 Story 单测与 4 项集成测试、WASM、预览源码校验和 911 个静态地址通过。专项 GPUI 测试覆盖加载时内容未挂载、重载重启错峰、实测高度缓动和减少动效即时切换几何。注册 JSON 与本地源码两条安装路径的文档用法均通过原生和 WASM 编译。源码 SHA-256 `a51c44af03dbb3e55b0a289d8f2d2d6854d93ddde756435c1d8a67ce50bd2eff`，注册包 SHA-256 `db920f5c7271d76c2fc5849e2b4b98d8b2781a2ecb42e0a28640680f807fbe35`。
- KUI 采用自有示例与 8px 矩形圆角，以临界阻尼和交叉淡入实现占位显现。官网内容带有从 4px 到 0px 的模糊过渡。当前 GPUI 实现只有淡入，并以 `Role::Status` 提供加载名称，官网的 `aria-busy` 对应语义待验证。原生 GPU、Safari、真机触摸、读屏、同速逐帧与帧耗时继续待验。

### Stretch refresh · 2026-11-10

- 公开 Markdown SHA-256 `621119537a7e70047da5870d615dd39cdfc1421415929d590bfea50cc9cf8704`，公开安装地址返回 404，未取得 Pro 源码。已核对标题、副标题、可自定义列表／键／渲染器、异步刷新、阻尼后 72px、结果与错误文案、可访问名称及根样式。Rust 提供 `set_items`、`set_subtitle` 和 `StretchRefreshResult`。[官网预览](https://uiarc.dev/components/stretch-refresh) · [公开参数](https://uiarc.dev/components/stretch-refresh/markdown) · [KUI API](../items/stretch-refresh.md)
- 官网在 1540px 下实测卡片 376×368px，顶栏 65px、刷新键 36×36px、列表可见高 301px；加载图中轨道约 72×2px，列表下移约 102px。40px 鼠标拉动释放保留 7 条，约 257px 拉动进入加载，随后增至 10 条。官网圆角、英文样例和图片与 KUI 有意区分，KUI 使用 8px 矩形圆角、自有演示数据和仓库原有图片。阻尼和动画曲线依据实测重建，精确参数尚待核对。[官网空闲](/private/tmp/kui-stretch-ref/arc-idle.png) · [官网拖动后](/private/tmp/kui-stretch-ref/arc-drag.png) · [官网结果](/private/tmp/kui-stretch-ref/arc-success.png)
- 浏览器亮色验证短拉回弹、长拉后显示刷新线和 3 条新动态、滚轮滚至底部与顶部时外页滚动量保持 0、回到顶部后另一手势触发、键盘焦点下空格触发。复核中发现头像渐变角溢出、统一故事容器撑到 520px，已修正独立头像圆形和预览高度。最终预览 `8b80bc89fc1d716e64b6`，实测 428px 画布、亮暗主题，以及 320px 中文版刷新后显示 3 条新动态且页面无横向溢出。[长拉加载](/private/tmp/kui-stretch-ref/kui-pull-refreshing.png) · [内滚动](/private/tmp/kui-stretch-ref/kui-scroll-inner.png) · [滚轮结果](/private/tmp/kui-stretch-ref/kui-wheel-result.png) · [空格结果](/private/tmp/kui-stretch-ref/kui-refresh-via-space.png) · [最终暗色](/private/tmp/kui-stretch-ref/kui-final-dark.png) · [最终窄屏](/private/tmp/kui-stretch-ref/kui-zh320.png) · [窄屏刷新结果](/private/tmp/kui-stretch-ref/kui-zh320-result.png)
- 格式、Clippy、341 项组件测试、112 项 Story 单测及集成测试、原生／WASM、预览源码校验及 907 个静态地址通过。专项测试覆盖鼠标短拉、越过阈值、内滚动回顶部、取消触摸、去抖后的滚轮释放、重复触发、错误及重试。注册 JSON 与本地源码的原生／WASM 安装示例均通过。源码 SHA-256 `a2f4e0992473f630dd4b3ae13a9658d4edc7339a395aad16eeb9042aebe30b5f`，注册包 SHA-256 `943d94203ecc8c4a39dcdceee38abd3199ba8bc1e0e16424b737042ae1a24c7a`。
- 仍需核对 Safari、原生 GPU、真机触屏和触控板、读屏忙碌语义、减弱动效、同速逐帧和帧耗时。浏览器画面与模拟事件证明部分操作路径，保留整体对齐待验。

### Charge session · 2026-11-10

- 公开 Markdown SHA-256 `2b7a56ed01fb739f52164ab173802d63f73e9a31eb2b178a1a8722e9aac5ac87`，公开安装地址返回 404，未取得 Pro 源码。已核对默认 77 kWh、150 kW、34% 电量、80% 限值、50% 最低值和降功率点、0.42 USD／kWh、en-US、速度 1 与默认播放。速度 0 保留粒子、暂停冻结粒子与充电进度；GPUI 事件映射到公开的限值与状态通知。货币字段支持宿主格式化回调。[官网预览](https://uiarc.dev/components/charge-session) · [公开参数](https://uiarc.dev/components/charge-session/markdown) · [KUI API](../items/charge-session.md)
- 官网 1540px 实测卡片约 761×708px，场景 695×167px，曲线绘图区 408×163px，指标网格 263×213px，滑块命中区 64×44px，操作键 182×44px；390px 浏览器实测卡宽约 304px，曲线移到指标上方。KUI 宽屏卡宽 761px，使用 8px 矩形圆角。曲线根据公开 SVG 可视采样重建，以 48 段积分算出默认 23 分钟。粒子速度、灯带质感与停充衰减来自公开说明与实测，尚无同速逐帧通过证据。[官网充电](/private/tmp/kui-charge-ref/arc-charging.png) · [官网手机](/private/tmp/kui-charge-ref/arc-mobile.png) · [测量数据](/private/tmp/kui-charge-ref/arc-layout.json)
- 官网实际操作确认停止后仍可改限值，恢复时当前电量已经越过限值会进入完成，新会话回到 34% 并保留 50% 限值。End 到 100%，Shift＋方向键变化 10%。本地 Web `4a00ca9a72bf402ca92d` 已验证停止 129 kW→0 kW、曲线拖动至 53%、滑块 Home 至 50%、完成后重开回 34% 且保留 50%。GPUI 测试覆盖默认数值、降功率、重复键不重复发事件、稳定按钮边界及模拟触屏纵横方向。[停充中间帧](/private/tmp/kui-charge-ref/kui-final-draining.png) · [0 kW](/private/tmp/kui-charge-ref/kui-final-stopped.png) · [53%](/private/tmp/kui-charge-ref/kui-final-drag-limit.png) · [焦点](/private/tmp/kui-charge-ref/kui-final-focus-50.png) · [重新充电](/private/tmp/kui-charge-ref/kui-final-new-50.png)
- Web 预览原可见性会在焦点滚动时误判。实测浏览器有 318 次动画帧回调，GPUI 画布停在第 23 次绘制和 139 kW。已按外层页面边界重新计算，画布滚出后再返回，2.8 秒等待期间能量从 0.2 kWh 仅变为 0.3 kWh；切代码页 3.2 秒后从 0.2 到 0.7 kWh。全程充电本应每秒增加约 1.67 kWh，取样显示离开期间未累计正常播放量。隐藏事件后清除监听，保留预览窗口与进度。[屏外前](/private/tmp/kui-charge-ref/kui-screen-before.png) · [返回后](/private/tmp/kui-charge-ref/kui-screen-after.png) · [代码页返回](/private/tmp/kui-charge-ref/kui-after-code.png)
- 格式、Clippy、338 项组件测试、112 项 Story 测试、原生／WASM 编译、预览源码校验及 903 个站点地址检查通过。注册 JSON 和本地源码安装两条路径的 Rust 示例均通过原生和 WASM 编译。付款流程仅将相同的货币格式化逻辑移至共享原语，两条安装路径同时复验。
- 320px 中文页面无横向溢出，复核充电提示的换行和操作键。从中文充电会话切到付款流程再返回，两个页面均报告正确的已绘制示例，复用同一预览地址。最终本地预览 `e89c30dba3b12d73a374`，组件 SHA-256 `36e9b12e1417e4078ad888126ca7b62eea1e1094fae5828be46ecbe8cdcf35d9`，注册包 SHA-256 `a1201c506a77f1d3dc1ca61a0ceb1d8a6d0e6ee761affc81f7060f76881ea49e`。Safari、原生 GPU、真机触屏、读屏、减少动效及同速逐帧和帧耗时仍待验收。[暗色](/private/tmp/kui-charge-ref/kui-final-dark.png) · [320px 中文](/private/tmp/kui-charge-ref/kui-narrow-zh.png)

### Payment flow · 2026-11-10

- 公开 Markdown SHA-256 `9b491fdde27e86dbbe8383c68c0c1520f448831b9824ec6f31596693751a01a9`。公开注册路径返回 Page not found，未取得 Pro 源码。默认额 $42.50、手续费 $1.53、期初余额 $1,284.10、USD、en-US、授权读数 1340ms、速度 1、显示步骤且未暂停。已核对五个节点及覆盖字段、状态和步骤文案、金额格式、强调色、说明和根样式。自定义图形使用 GPUI 元素工厂，说明为文本，完整区域格式由宿主格式化回调提供。[官网预览](https://uiarc.dev/components/payment-flow) · [公开参数](https://uiarc.dev/components/payment-flow/markdown) · [KUI API](../items/payment-flow.md)
- 官网在 1540×930 下实测图形约 761×187px，桌面卡片 150×60px，主行步长约 203px，结算轨道低 116px。手机图形 342×391px，四个节点竖排，余额在右上。按公开约定 700px 改轨道方向、420px 堆叠进度。官网卡片圆角实测 16px，KUI 矩形卡片及金额标签用主题 8px。默认图标按公开预览绘制。授权、清算、手续费及入账的时间点来自浏览器取样，曲线按 Rust 实现调节。[官网桌面](/private/tmp/kui-pay-arc-wide.png) · [官网手机](/private/tmp/kui-pay-arc-mobile.png) · [时序采样](/private/tmp/kui-pay-arc-timeline.json)
- 本地 Web `9388bcb2e7d3713b5d5e`。桌面亮色取样看到 $42.50 筹码沿结算轨道到处理商、$1.53 手续费、余额由 $1,284.10 到 $1,325.07 且下轮保留，以及授权筹码返程和 1.34s 授权读数。暗色和 320px 中文布局已查看，窄屏可见四个节点、余额和两条完整进度，页面无横向溢出。恢复窗口宽度时首个主题切换在 3 秒内未生效，页面稳定后再次点击生效，主题切换与 Safari 仍待专项复测。[手续费帧](/private/tmp/kui-payment-flow-ref/kui-cycle-1.png) · [入账帧](/private/tmp/kui-payment-flow-ref/kui-cycle-3.png) · [授权返程](/private/tmp/kui-payment-flow-ref/kui-cycle-7.png) · [暗色](/private/tmp/kui-payment-flow-ref/kui-wide-dark.png) · [320px 中文](/private/tmp/kui-payment-flow-ref/kui-mobile-dark-zh.png)
- 格式、Clippy、334 项组件测试、112 项 Story 单测及集成测试、原生／WASM 编译、预览源码校验和 899 个静态地址检查通过。注册 JSON 与本地源码分别安装，文档示例在两条路径均通过原生和 WASM 编译。专项测试覆盖处理商扣费位置和跨轮余额连续性。源码 SHA-256 `9b13fc1a3288eed3bbc19440209e8f626d0e9f58373020981e62ea117f15d1e6`，注册包 SHA-256 `2a57803262a5088393e3ac1dc10cced636d186b93a275348e8bd45b975051b5d`。
- 仍需核对官网和本地同速逐帧、全部覆盖参数、悬停减速、暂停恢复、真实减弱动效、Safari、原生窗口、读屏、真机触摸和帧耗时。取样能核实流程顺序和金额状态，完整体验保持待验。
- 充电会话接入后共用同一份货币格式化函数。付款流程逻辑未变，注册 JSON 与本地源码双路径再次通过原生与 WASM 示例编译。新版源码 SHA-256 `35b1452e5aa4aaf8931ba8aea639da907efcf87e2b00e722ef2a90b71e3913da`，新版注册包 SHA-256 `1e297f148676bc2321ce4d28d57f49b0ed05ad73e3d15bc9a26f936519c639fd`。

### Dot grid · 2026-11-10

- 公开 Markdown SHA-256 `819526502880dae57796f6793eec9d43f1d5cf9f9ba5d03782aedd97afc7cf0a`，公开注册表返回 404。核对了默认点距 22px、点半径 1.4px、透镜半径 150px、峰值推移 16px、边缘淡出、涟漪、两种颜色、内容和根样式。GPUI 的 `with_id`、`with_style`、`with_content` 与 Rust 选项映射到这些能力。点阵是装饰层，内容位于独立视图并保留自己的交互。[官网预览](https://uiarc.dev/components/dot-grid) · [公开参数](https://uiarc.dev/components/dot-grid/markdown) · [KUI API](../items/dot-grid.md)
- 官网 1440px 浏览器实测外框约 668.8×420px，画布可见面积约 666.8×418px，底部三档组合 234×36px，每键 76×30px。截图像素峰值的横向步长为 14／22／32px。官网手机外框高 380px。KUI 采用相同点距和面板高度，矩形圆角按 KUI 8px。[官网亮色](/private/tmp/kui-dot-grid-ref/arc-rest-light.png) · [官网暗色](/private/tmp/kui-dot-grid-ref/arc-rest-dark.png) · [官网手机](/private/tmp/kui-dots-arc-mobile.png)
- GPUI 专项 4/4，核对三档居中重排、透镜峰值和移出后停止帧需求、六波叠加上限、屏外暂停、减少动效静止着色，以及模拟触摸空白区跟手与内容按钮各收到一次点按。涟漪采用公开的约 620px/s 和 1.5s，弹簧系数、波宽和淡出曲线为 KUI 实现估值。内容与绘制分层，连续相同 GPUI 图元由渲染器批处理。
- 最终 Web 版本 `674a5f796c5e51f70aa6`。鼠标实测三档，Tab 后按 Enter 从紧密回到默认。取样截图测到 14／22／32px。固定点重复点击相对稳定透镜画面取四帧，透镜半径之外的亮度差至少 12 的像素依次为 0、6、73、318，观察到涟漪向外推进。截图只用于状态与方向复核，不能验证 60fps。移出后恢复规则点阵，归位截图的纵向采样有约 1px 偏差。暗色与 320px 中文版均可读，窄屏画布宽 288px，页面无横向溢出。[紧密](/private/tmp/kui-dot-grid-ref/kui-dense.png) · [键盘默认档](/private/tmp/kui-dot-grid-ref/kui-keyboard-default.png) · [疏朗](/private/tmp/kui-dot-grid-ref/kui-loose.png) · [点击帧](/private/tmp/kui-dot-grid-ref/kui-wave-3.png) · [暗色](/private/tmp/kui-dot-grid-ref/kui-dark.png) · [320px 中文](/private/tmp/kui-dot-grid-ref/kui-zh-320.png)
- 格式、Clippy、332 项组件单测、108 项 Story 单测及集成测试、WASM、预览源码校验和 895 个站点地址检查通过。从注册 JSON 与本地源码分别安装，文档用例通过原生及 WASM 编译。源码 SHA-256 `ff03a8a324d1548ea7db454223dd54c0ecc4091942e00814d208d39d4ec5c91b`，注册包 SHA-256 `ca95df38de4c25a227a283e25c3836610f5ad00ea67ef4255be7d5e56870cb8e`。
- Pro 源码未取得。KUI 大画布会限制点数，空白区拖动会捕获直接触摸，内容区保留自身手势。渲染使用宿主的 DPI。官网波形、同速逐帧、Safari、桌面 GPU、读屏、真机触摸与滚动、帧耗时仍需核对。

### Branch graph · 2026-11-10

- 公开 Markdown SHA-256 `12250e1441de6b060356ddd46caeb88f2332de32621c8a577c6b470b46bade03`，公开注册表返回 404，官网 Pro 安装区需授权。核对了提交及作者字段、分支和色值、合并请求、受控策略及回调、默认策略、策略顺序、标题、自动播放、速度、暂停、强调色、无障碍名称与根样式。Rust 用 `BranchStrategyChange` 回传用户请求，用 `set_strategy` 应用宿主回写。[官网预览](https://uiarc.dev/components/branch-graph) · [公开参数](https://uiarc.dev/components/branch-graph/markdown) · [KUI API](../items/branch-graph.md)
- 官网页宽 1536px 的合并前态实测面板约 757px，策略栏约 283×38px，列表 737×362px，行高 44px。轨道中心 x 为 22／40／58，首个点中心 y 为 28，描边 2px。分支在父节点端转弯，合并在子节点端收拢。核对亮暗两套 `oklch`，转为 KUI 使用的 sRGB。实测手机预览区域 342px 时保留名称和分支标签，隐藏头像、哈希与时间。[官网桌面](/private/tmp/kui-branch-arc-before.png) · [官网手机](/private/tmp/kui-branch-arc-mobile.png)
- 默认示例合并前 8 行，领先和落后均为 3。合并为 9 行，新增提交有两个父节点。变基仍为 8 行，三个提交移到 main 并重写哈希。压缩为 6 行，聚合 +521／−157／19 个文件。KUI 预览哈希由稳定算法生成，与官网的演示哈希不同。没有仓库写操作。
- 官网连续位移粗采样包含输入延迟。变基在动作调用后约 271–1110ms 仍有观测变化，压缩为约 277–893ms。合并的首次样本已停稳，无法据此确定时长。KUI 参数为当前实现估值，未做同速帧对帧验收。
- GPUI 专项 6 项覆盖提交图、合并统计、受控回写、窄屏键盘、轨道转折及动画反向接续。取消或重定向从可见行坐标和边透明度开始。宿主可见性控制停帧，静止期间由单个定时任务唤醒自动播放。

- 最终预览 `8a83916c45898b9de0c2`。实测切换合并 → 变基 → 压缩并回到合并前，分别显示 9、8、6、8 条提交，画布保持 564px 高。修正了策略已更新、旧提交仍停在画布的问题。预览为最多一行额外空间预留高度。活动动画请求下一帧，34ms 内没有新画面时补一次唤醒。用键盘 Home 打开详情、Esc 关闭，亮暗两版保留当前策略。[合并](/private/tmp/kui-branch-graph-ref/kui-merge.png) · [变基](/private/tmp/kui-branch-graph-ref/kui-rebase.png) · [压缩](/private/tmp/kui-branch-graph-ref/kui-squash.png) · [详情](/private/tmp/kui-branch-graph-ref/kui-before-detail.png) · [暗色](/private/tmp/kui-branch-graph-ref/kui-dark.png)
- 320px 中文视口内画布宽 288px，页面无横向溢出。变基首行保留提交标题和两枚分支标签，说明换行可读，详情内保留作者、时间、哈希及统计。[窄屏](/private/tmp/kui-branch-graph-ref/kui-zh-320.png) · [窄屏详情](/private/tmp/kui-branch-graph-ref/kui-zh-320-detail.png)
- 格式、Clippy、328 项组件单测、108 项 Story 单测及集成测试、WASM、预览源码一致性和 891 个站点地址通过。从注册 JSON 和本地源码分别安装，文档示例通过原生和 WASM 编译。源码 SHA-256 `bd9bb542ea06d6031bc09202928d062ea2f9792e46d2709afd114b1c989bc41e`，注册包 SHA-256 `f9a2443241555676f33dde9d48e52bd7feaa843900a99020041fa0583d7be0b3`。
- 当前采用 KUI 8px 矩形圆角、渐变头像、窄屏双行标签与模拟哈希。窗口在最后一行落位后才释放预留高度。公开资料未提供 Pro 实现源码。原始动效参数、同速逐帧、Safari、桌面 GPU、读屏、真机触摸和帧预算仍待核对。

### DNS resolution · 2026-11-10

- 公开 Markdown SHA-256 `1ac7b119f713dac5dadc7cfae0300a0a2714cd4e636e372b1cab6b7f2163b341`，公开注册包返回 404。核对了域名、地址、TTL、查询与缓存耗时、五类节点覆盖、速度、暂停、强调色、控件、说明、无障碍名称和根级样式。GPUI 的 `with_id`、`with_style` 和自定义节点元素工厂承接 Web 的根级样式及图标扩展。默认 `app.acme.dev`、`203.0.113.24`、300s、84ms、1ms。[官网预览](https://uiarc.dev/components/dns-resolution) · [公开参数](https://uiarc.dev/components/dns-resolution/markdown) · [KUI API](../items/dns-resolution.md)
- 官网可见矢量实测以 640px 为横竖布局分界，视图坐标分别为 `1000×612` 与 `390×676`，四条路线是二次曲线。桌面阶段键高 44px、播放键高 36px，缓存卡宽 184px。读取六段说明、状态和实际绘制位置，核对跳转后保持原播放状态。从同一次播放进行粗采样，阶段约在 2.0／4.2／6.4／8.6／11.1 秒切换，缓存约 15.0 秒过期，约 16.0 秒回到首段。结果包含输入和采样延迟，KUI 用一位小数的估值；单个数据包、TTL 曲线和弹簧系数仍待连续帧对照。[官网缓存画面](/private/tmp/kui-dns-resolution-ref/arc-wide.png)
- 共享 Rust 使用同一个时钟推进状态、轨迹、缓存记录和步骤条。GPUI 专项 4/4，覆盖阶段、过期及回环、弧长表、默认参数及覆盖、宽窄布局、方向键和 Home/End、播放中和暂停时跳转。站点宿主按预览的可见性停止帧请求。减少动效时渲染静止阶段。

- 最终 Web 版本 `62e45c2568a5fd3954a9`。浏览器实测暂停后立即点步骤再按 End，停在第六段首帧，TTL 198s，下一次请求的浮签尚未出现。最终版已在亮暗主题及 320px 中文视口检查，画布宽 288px，文档无横向溢出。修复浏览器节点标签贴上平台以及浮签盖住文字，浮签在交叠区淡出，数据包路径保持连续。[英文亮色](/private/tmp/kui-dns-resolution-ref/kui-final-wide.png) · [英文暗色](/private/tmp/kui-dns-resolution-ref/kui-final-dark.png) · [320px 中文](/private/tmp/kui-dns-resolution-ref/kui-final-zh-320-cache.png)
- 格式、Clippy、322 项组件单测、108 项 Story 单测及其集成测试、WASM、预览源码校验和 887 个站点地址检查通过。独立项目从注册 JSON 和本地源码两条路径安装并编译文档用例，原生和 WASM 均通过。组件源码 SHA-256 `4235f831595502777673526ca6ed26423856a1f3ef9bf0e09a653998fc3956b6`，注册包 SHA-256 `e4bcea22d4c23966b47eb662cf232d12fbbf9b9e77b9273dd6efa886f9456f27`。
- 公开材料没有提供 Pro 实现源码。当前保留 KUI 8px 矩形圆角和字体，节点材质和图标为 GPUI 绘制，预览内宽与官网截图不同，窄屏文字避让由 KUI 处理。逐像素、单包同速逐帧、Safari、桌面 GPU 实拍、读屏、真机触摸和帧时间继续待验。

### Booking pill · 2026-11-10

- 公开 Markdown SHA-256 `54d90c9350300a0c6d4cb70573ed922c3d695e21639d6bca7e0b3af2396cf411`。公开注册包返回 404，官网 Code 面板要求 Pro。默认 21 天、1–12 人、初始 2 人、17:00–22:30 每半小时、预选 19:30，12／24 时制只改显示。KUI 使用 `NaiveDate`、`NaiveTime` 和 `BookingStepChanged`，可订判定和确认任务由宿主提供。[官网预览](https://uiarc.dev/components/booking-pill) · [公开参数](https://uiarc.dev/components/booking-pill/markdown) · [KUI API](../items/booking-pill.md)
- 官网浅色实测收起约 164.4 × 52px、人数 300 × 60px、日期 420 × 124px、时段 360 × 262px、确认票据 340 × 215px。日期滑轨 60px 高，卡片间距 52px，遮罩内侧停点为 14%／86%。时段四列，普通按钮 80.5 × 40px，返回与主操作 44px 高。票据外框半径 30px，24px 背景模糊。Page Down 后日期从 9 月 22 日到 29 日，Enter 打开时段，20:00、20:30、21:00 禁用，19:30 已选。[收起](/private/tmp/kui-booking-pill-ref/start.png) · [人数](/private/tmp/kui-booking-pill-ref/party.png) · [日期](/private/tmp/kui-booking-pill-ref/date.png) · [时段](/private/tmp/kui-booking-pill-ref/time.png) · [票据](/private/tmp/kui-booking-pill-ref/review.png)
- 官网确认按钮点击被自动审批拒绝，原因是可能向商家提交真实预订。保留公开文档作为成功态行为依据。KUI 本地示例只模拟返回结果，GPUI 专项 4/4，覆盖关闭日期、不可订时段及预选回退、窄屏日期翻周、确认前发现原时段失效、失败保留票据、连续确认只调用一次和同票据重试成功。
- 浏览器走查收起、人数、日期、时段、票据及本地模拟成功。Page Down 从 9 月 22 日到 29 日，Enter 打开时段，Right 从 19:30 越过三项禁用时段到 21:30，成功回执保留相同日期、时间和人数。320px 中文视口的 288px 画布中，亮暗两种时段页都完整保留四列。[320px 浅色](/private/tmp/kui-booking-pill-ref/kui-zh-320-time.png) · [320px 暗色](/private/tmp/kui-booking-pill-ref/kui-zh-320-dark.png) · [模拟确认中](/private/tmp/kui-booking-pill-ref/kui-pending.png) · [模拟成功](/private/tmp/kui-booking-pill-ref/kui-booked.png)
- 最终版 `8bcd16c2aa14db8066f3` 修复首次人数滚动与面板尚窄时露出半个箭头。保留 KUI 8px 矩形圆角和半透明面板。过渡中先展开轮廓再显露控件，官网背景模糊、票据撕口和连续动效仍待对照。Safari、读屏、真机触控与桌面 GPU 画面待验。[最终中文票据](/private/tmp/kui-booking-pill-ref/kui-final-review.png) · [最终人数](/private/tmp/kui-booking-pill-ref/kui-final-party.png)
- 格式、Clippy、318 项组件与 108 项 Story 单测、WASM、预览源码校验和 883 个静态地址检查通过。注册 JSON 与本地源码两条独立安装路径通过原生与 WASM 文档示例编译。组件源码 SHA-256 `a61105648bf4e7f9ae4a09577ee7063d7710fa42f58d3acbfb7401a8011c04f4`，注册包 SHA-256 `367db8a049cf7e74e160d3d96abd18a99bb84ebe018bb6aa1de4d54b8de37082`。

### Time dial · 2026-11-10

- 公开 Markdown SHA-256 `bd7e34ee1ad4d712af8a5bed32ed85cbfa06ff8061b77a654d64ccca52d19c27`。公开注册包返回 404，官网 Code 面板明确要求 Pro。公开参数核对覆盖城市、UTC 时刻和主城市的受控与非受控模式、边界、星期、12／24 时制、工作时段和标签。KUI 通过两个事件与宿主回写，样式与可见性由 builder 接入。[官网预览](https://uiarc.dev/components/time-dial) · [公开参数](https://uiarc.dev/components/time-dial/markdown) · [KUI API](../items/time-dial.md)
- 官网 1280px Fit 版实测表盘 256px、城市列 420px、间距 20px、行高 52px、行距 2px。使用 300 单位坐标，轮缘半径 146、工作扇面外 130 内 60、主轨 106、从轨 88／77／66。移动版 iframe 内宽 390px，表盘仍为 256px，城市列 342px，有跨日标签的行约 66.39px。取样了亮暗主题的轮缘、底色、工作扇面、轨道虚线和选中行。[官网浅色](/private/tmp/kui-time-dial-ref/light.png) · [官网深色](/private/tmp/kui-time-dial-ref/fit.png) · [官网移动](/private/tmp/kui-time-dial-ref/mobile.png)
- 官网交互实测纽约 17:00，Right 到 17:15，Page Up 到 18:15。切伦敦后读 23:15，其他城市时刻保持原值。重叠动作选择伦敦 17:45，三个办公室在工作。End 后切东京显示星期二，纽约和旧金山标前一日。点击正午刻度落在 12:00。KUI GPUI 专项 5/5，覆盖 12／24 时制、跨日、边界弹性、松手单次事件、受控回写、键盘和响应式。同一时刻的最近重叠专项通过。
- 本地浏览器实测同一组键盘步进、伦敦切换、重叠时段、重置和正午点按。320px 中文画布宽 288px，表盘内宽 240px，四城时刻、跨日标签和两个操作均可见。亮暗对照发现 AM／PM 换行，改为按字体测量后缀并固定在同一行。[浅色英文](/private/tmp/kui-time-dial-ref/kui-meridiem-fixed.png) · [暗色英文](/private/tmp/kui-time-dial-ref/kui-dark.png) · [中文 320px](/private/tmp/kui-time-dial-ref/kui-zh-320.png) · [最终中文](/private/tmp/kui-time-dial-ref/kui-final-zh.png)
- 官网从正午沿顺时针快拨的同起终点试验，按当次手势速度分别落在次日 11:00、12:45、20:15。KUI 调整初版三小时惯性上限后，同路线一次落在次日 04:15，Home 返回当地次日 00:00。KUI 的惯性仍短于该组官网试验，内部弹簧参数未公开，尚需同速输入与中间帧核对。保留 KUI 的 8px 矩形圆角。真机触控、Safari、读屏播报、桌面 GPU 画面和帧耗时待验。[快拨跨日](/private/tmp/kui-time-dial-ref/kui-flick-final.png) · [Home 到当地午夜](/private/tmp/kui-time-dial-ref/kui-home-final.png)
- 最终构建 `28f23a838e5a568ab7ce` 通过格式、Clippy、314 项组件与 108 项 Story 单测、原生与 WASM 编译、预览源码校验和 879 个静态地址检查。重新构建 CLI 后分别从注册 JSON 和本地源文件独立安装，文档示例在两个目标均通过编译。Rust 源码 SHA-256 `5e4a5562a87013fe10df473971414b514cdccb205217b8a9ea46cf8ac0810c3e`，注册包 SHA-256 `ec5e0e27acd18db33440aea7559f5b8ab632da386cee7768266d59e237521357`。

### Keycap keyboard · 2026-11-10

- 公开 Markdown SHA-256 `7a2e92c510e17ba43252112c75666b281e7952b8f038f01c88fe1c7adf54389d`。公开注册包返回 404，Code 面板要求 Pro。核对 Compact、Letters、自定义键位、受控值、280 字长度、焦点和窗口监听、输出显示和标签。官网英文暗色 Fit 版键盘约 521.52 × 200.14px，单键 29.28px，键距 4.10px、按压位移 2.49px。公开节奏为按下 35ms、复位 200ms 和余光 700ms。[官网预览](https://uiarc.dev/components/keycap-keyboard) · [公开参数](https://uiarc.dev/components/keycap-keyboard/markdown) · [KUI API](../items/keycap-keyboard.md)
- GPUI 专项 5/5，覆盖行宽、快速反按、重叠输入源、Shift 与系统快捷键、组合字符删除及切换布局保留文本。WASM 预览 `1a96a1e3bc8243a8b806`。原生、WASM、注册 JSON 和本地源码安装通过，组件源码 SHA-256 `7c3489c8a7b54d394dc31c5b9ca11b54bb2d0333ec68f9c1ce5bd4c5bde591e9`，注册包 SHA-256 `05108d6988feb3b73e272e9874a68206fc137cb60e212ff7327e54fc4d510f43`。
- 中文浏览器实测 Shift+a、b、Backspace、点击 q 后得到 `Aq`，切换 Letters 后保留同一内容。[完整键盘](/private/tmp/kui-nov09-review/verified-keycap-keyboard.png) · [字母键盘](/private/tmp/kui-nov09-review/verified-keycap-keyboard-letters.png)
- 窗口监听默认只观察未被其他控件消费且没有 GPUI 焦点的按键，宿主可为已知非文本控件设置过滤条件。GPUI 归一化后不区分左右修饰键。KUI 预览默认 Compact，官网窄屏默认 Letters。还需同尺寸、同速对照、Safari、原生实拍和真机触摸。

### Date reel · 2026-11-10

- 公开 Markdown SHA-256 `d8ea952856dad2e1fa91dac7cc0330846ef03ce817b157ec77c9a754308ef1df`，公开注册包返回 404，Code 面板要求 Pro。核对日期、时间和日期时间三模式、初值、受控回写、停稳事件、日期上下限、五档分钟步长、12／24 时制、相对日期与快捷项、可选确认、覆盖标签、入场、速度、暂停、强调色与样式。KUI 使用 `NaiveDateTime`，内建中英文，其他语言由宿主格式函数提供。[官网预览](https://uiarc.dev/components/date-reel) · [公开参数](https://uiarc.dev/components/date-reel/markdown) · [KUI API](../items/date-reel.md)
- 英文暗色官网 Fit 版实测 520 × 438.69px，滚轮区高 235.2px、字号 22px、选中带高 42px、确认键 127.4 × 36px。移动版滚轮区高 201.6px、字号 18px、选中带高 36px，确认键 44px 并另起一行。以选中行居中，下一行实测位移 41.36px，竖向压缩约 0.9257。KUI 用 `sin` 做行位移，用 `cos` 推导字形尺寸、淡出和右侧锚点，透镜用裁剪绘制。[官网桌面](/private/tmp/kui-date-reel-reference.png) · [官网移动](/private/tmp/kui-date-reel-reference-mobile.png)
- GPUI 专项 5/5。覆盖指数惯性中途反向保持位置和速度、有限边界回弹和循环列近路、闰年 2 月 29 日纠正、参考日变更保留选值、确认失败保留时间及重复提交守卫。Web 实测与官网同样从 9 点按 Down 到 10 点，Page Down +3 小时。滚轮只改分钟，正文滚动位置保持不变。选「This evening」落在当天 18:00，成功态按钮与初始状态同宽。早期浏览器测试发现拖动释放后确认键偶尔停在等待态，增加全局指针释放路径后，实际拖动可吸附到新值并重新确认。[方向键](/private/tmp/kui-date-reel-local-hour10.png) · [翻页](/private/tmp/kui-date-reel-local-next-key.png) · [滚轮](/private/tmp/kui-date-reel-local-wheel-only.png) · [完成确认](/private/tmp/kui-date-reel-local-scheduled.png) · [拖动回归](/private/tmp/kui-date-reel-local-drag-fixed.png)
- 最终 Web 版本 `ddfdd26bf74f86057b5d`。英文暗色实拍显示完整滚轮和选中带，键入 4、向右移焦、键入 45 后落在 4:45 AM。320px 中文亮色预览中画布宽 288px、卡片约 264px，日期简写完整，分钟拖到 9:25 后按 44px 高确认键，成功态保留 9:25。[英文暗色](/private/tmp/kui-date-reel-final-dark.png) · [键入定位](/private/tmp/kui-date-reel-final-typeahead.png) · [320px 中文](/private/tmp/kui-date-reel-final-320.png) · [320px 确认](/private/tmp/kui-date-reel-final-320-confirm.png)
- 格式、Clippy、5 项专项、107 项 Story 单测及集成测试、双目标编译、注册源码校验、212 个双语组件和 871 个静态地址通过。从注册 JSON 和本地源码分别在空白项目执行 `kui add date-reel`，两条路径的文档用法都通过原生与 WASM 编译。控件 SHA-256 `067e5a39b92d3f5788c572dcc0598828f5a345a580f68fdb68cca60e36957da7`，注册包 SHA-256 `044bfde7996605bbf65a7c6b4602cf5636ae19a281a502e0fe7a1e915073fd37`。
- 已知差异包括 KUI 8px 矩形圆角、内建两种语言与宿主格式函数，以及字号缩放和官网字形横纵独立形变的差异。惯性和吸附的内部参数按公开行为拟合。日期和纯时间模式的视觉、同速逐帧、Safari、原生窗口、真机触控、读屏及帧耗时仍待核对。

### Typewriter terminal · 2026-11-09

- 官网公开 Markdown SHA-256 `c8a21a873a3e7fd1af2f95ca4e36fdf6a8170d86411b1612e48bc23e2200ff56`。公开注册包返回 404，Code 面板要求 Pro。核对步骤、目录继承、暂停、文字／进度／任务行、时间倍率、循环、11 行默认高度、复制、强调色、名称、说明及容器样式。对应 Rust 类型、默认值和可编译示例记录在组件文档。 [官网预览](https://uiarc.dev/components/typewriter-terminal) · [公开参数](https://uiarc.dev/components/typewriter-terminal/markdown) · [KUI API](../items/typewriter-terminal.md)
- 英文暗色官网 Fit 实测约 521.61 × 369px，窗口高 309px，标题栏 44px，正文 263px、等宽字体 12px、21px 行高、16px 内边距。窗口到控制列留 16px，播放、拖动和回放区高 44px，轨道 4px，拇指 18px。全屏窗口宽约 760px。官网窗口圆角 26px，KUI 使用 8px；播放键和滑块保留各自的圆形。 [官网宽屏](/private/tmp/kui-terminal-ref-fullscreen.jpg) · [官网结束态](/private/tmp/kui-terminal-ref-settled-end.jpg)
- 官网 Home 落在 0 且屏幕清空，Arrow Up 到第一条命令约 0.30s，第二、第三条的刻度在轨道 40.55% 和 76.93%。End 显示最终提示行，Arrow Down 返回第三条命令开头。KUI 自有 Cargo 与 KUI 脚本长约 16.68s，前三个起点为约 0.30／6.74／12.79s。字形时序以命令为种子预编译，文本形状按内容变化缓存。拖动以当前输入位置直接派生屏幕，回放和键盘定位沿时间轴过渡。
- GPUI 专项 4/4，覆盖组合字符和 emoji 分界、shell 分类和链接、零间隔事件顺序与目录继承、暂停减速和 65ms 时反向回放保持位置、长词换行及减少动效。官网内部节拍、全部缓动参数与光标细节继续核对。
- Web 版本 `ee23a0b0adb27f98f864`。修复默认比例字体回退，预览选用仓库随附的 Fira Code，安装用户可通过 `with_font_family` 指定已注册的等宽字体。轨道起点距窗口左缘 56px，实测轨道约 330px。状态播报层移出布局，Home 与暂停后的 End 中窗口上下边缘和控件中心线相同。实测回拖到早期命令，复制后剪贴板精确等于三条命令按换行拼接。320px 中文浅色预览内 288px 画布，脚本自动换行，控制键和轨道完整。[键盘起始](/private/tmp/kui-terminal-local-aligned-home.jpg) · [暂停结束](/private/tmp/kui-terminal-local-aligned-paused-end.jpg) · [拖动](/private/tmp/kui-terminal-local-aligned-drag.jpg) · [320px 中文](/private/tmp/kui-terminal-local-320-zh.png)
- 格式、Clippy、Story 测试、WASM、注册源码校验及 211 个双语组件和 867 个地址检查通过。注册 JSON 与本地源码两条独立安装路径均通过原生和 WASM 文档示例编译。控件 SHA-256 `3862d2462114fdff7d9c4e23adfe32a1f7dd651e75270b5fa1a9ed061622fc85`，注册包 SHA-256 `d7a06cb02bd63449c5b43b2c872a9bb36a418f1a14d9386a27019e8896a05235`。当前差异包括 KUI 圆角、示例命令及字体。原生 GPU 画面、Safari、真机触摸、逐帧动效与帧耗时仍待验。

### Live visitors · 2026-11-09

- 官网公开 Markdown SHA-256 `4339f0d69613d8a009f23e38c61b258d9b76bad8b00c6f1d65e21acd0cc8dcd6`。公开注册包返回 404，Code 面板要求 Pro。逐项映射人数、页面、到访 ID/位置/种类、转化 ID/国家/时间/标记、连接状态、受控与非受控暂停、标签、列表上限、容器样式和样例启停。默认 `Live`、未暂停、显示暂停入口、每列四项、最大宽度 640px。宿主传整份快照，恢复时提交最后一份待更新数据。 [官网预览](https://uiarc.dev/components/live-visitors) · [公开参数](https://uiarc.dev/components/live-visitors/markdown) · [KUI API](../items/live-visitors.md)
- 官网实测 640px 为双列，521.61px 为堆叠，319.38px 为人数标签另起一行。堆叠版内边距 16px、地图 487.61 × 175.54px、数字 32px／500、数字条高 40px、暂停键 36 × 36px、列表行高 32px 加 4px 间距、国旗 16 × 12px。官方外壳 26px、暂停键 18px，KUI 按矩形圆角规范统一采用 8px。 [官网双列](/private/tmp/kui-visitors-ref-wide.jpg) · [官网堆叠](/private/tmp/kui-visitors-ref-paused.jpg) · [官网移动预览](/private/tmp/kui-visitors-ref-mobile.jpg)
- 按公开时长实现 280ms 数字／条形／排序、160ms 行淡入淡出、900ms 波纹、400ms 过期淡出、2s 城市名、1.6s 状态点循环及 100／160ms 状态文案过渡。样例每 600ms 到访、每 2.4s 生成转化，不联网。小屏事件时间单行右对齐，国旗使用 KUI 内嵌图形。地图坐标和陆地轮廓基于 Natural Earth 公共领域 1:110m 陆地 v5.1.2 的 100 × 36 网格离线生成，几何按卡面边界缓存。 [地理数据](https://github.com/nvkelso/natural-earth-vector/blob/v5.1.2/geojson/ne_110m_land.geojson)
- GPUI 专项 5/5。测试覆盖暂停和重连只提交最新数据、切换样例和真实数据时清理地图、进位时保持未变化数字、65ms 中途重排保留行位置，以及实测 640px 双列和 522／521.6／320px 堆叠。Web 预览暂停后连续两次截帧，人数、页面榜单和时间列在 RGB 通道容差 6 内均无像素变化。Enter 和空格实测恢复更新。 [暂停帧](/private/tmp/kui-visitors-local-paused.jpg) · [停留后](/private/tmp/kui-visitors-local-paused-held.jpg)
- 最终 Web 版本 `b8efbb8204a47c6a0a81`。1280px 英文暗色页面的 628px 画布内卡片正确堆叠，地图上滚轮使正文从 271.5px 移到 358px，滚动后空格恢复更新。独立宽屏稳定画面中，卡宽 640px、两列表宽约 297px、间距约 24px。320px 中文亮色页的画布宽 288px，数字说明另起一行，时间列单行右对齐。 [英文堆叠](/private/tmp/kui-visitors-local-verified-stacked.jpg) · [宽屏双列](/private/tmp/kui-visitors-local-verified-wide.jpg) · [320px 中文](/private/tmp/kui-visitors-local-verified-320.jpg)
- 格式、Clippy、Story 测试、WASM、预览源码校验和静态站检查通过。验证了 210 个双语组件和 863 个静态地址。用 `kui add live-visitors` 从注册 JSON 和本地源码分别安装到独立项目，文档首段示例通过原生与 WASM 编译。Rust 文件 SHA-256 `2037abcfa7d9dc556e081455d5ae00964c9f10b8a52208f82386814563e08ff2`，注册包 SHA-256 `30db7d463d0c68afafc132df524280c1da3a151b22c9d5afbc4d5665dfa1fbd4`。
- 当前已登记的差异是 8px 外壳与暂停键、地图投影和字形、独立生成的样例流。官网内部颜色和缓动曲线、同速连续帧、60Hz 帧预算、Safari、桌面 GPU 窗口、真机触摸、读屏及 125% 显示比例待验。本机 `xcrun metal --version` 显示缺少 Metal Toolchain，原生编译记录与桌面 GPU 实拍分开。
- 修正视口宽度未初始化时的布局回退。新版独立预览首次可见截图显示 640px 卡片、完整地图和两列列表。该截图只验证单次冷加载可见结果。双安装入口仍通过原生和 WASM 编译。源码 SHA-256 `6c8d98d456b984fce8265bbee700e163e124e6e30b78fd40dde41e4b7ce7c975`，注册包 SHA-256 `2e564270cc04e0154c4e3c6319d6a5373bd95fe203a359beb2327ca249663a18`。[首次可见截图](/private/tmp/kui-terminal-live-cold.png)

### Holo card · 2026-11-09

- 核对公开 Markdown 的正面必填、背面可选、受控与非受控、翻面回调、默认最大倾斜 14°、强度 1、方向传感器默认开启、可访问名称及 300px 默认宽度。公开 Markdown SHA-256 `5890e50a785d5529dcb9f61872094c84e5cf7923918a48909cf0ca322050363c`。公开注册包返回 404，Code 面板要求 Pro 权限。Rust 构造函数、默认值和事件时机记录在单组件文档。 [官网预览](https://uiarc.dev/components/holo-card) · [公开文档](https://uiarc.dev/components/holo-card/markdown) · [KUI API](../items/holo-card.md)
- 官网实测正面 290 × 406px、圆角 22px、透视距离 900px。核对沿 118° 移动的镭射层、120px 重复闪点和跟随指针的高光。Enter 实测翻到背面，`aria-pressed` 与可读卡面一并更新。 [官网正面](/private/tmp/kui-holo-ref-front.jpg) · [官网背面](/private/tmp/kui-holo-ref-back-confirmed.jpg)
- KUI 共用 GPUI 控件使用 5:7 比例、8px 圆角、Kense 标识和原创卡面。两个弹簧分别跟踪倾斜与反光，翻面中途更改目标保留瞬时角度和角速度。键盘焦点描边跟随投影轮廓。两端接收相同的触摸输入和可选传感器值。减少动效去掉倾斜、旋转和闪点，绘制卡面保留交叉淡入。公开示例提供方向传感器受信任点按授权及手指拖动回退。
- 预览版本 `2ae7fc413986bc3d7269`。英文暗色实测点击翻到背面、Enter 回到正面，倾斜时 2px 焦点描边贴合四边。双击后仍为正面。中文亮色实拍完整。前一构建的 320px 中文视口中，预览宽 288px、卡片宽 260px，标题与两列读数完整。本轮浏览器日志曾报告英文预览字体回退，最终字形一致性继续核对。 [正面](/private/tmp/kui-holo-final-zh.jpg) · [背面](/private/tmp/kui-holo-final-back.jpg) · [焦点](/private/tmp/kui-holo-final-focus.jpg) · [320px](/private/tmp/kui-holo-local-zh-320.jpg)
- 专项测试 3/3，覆盖透视收边、反向保持角度与速度、受控确认及键盘操作。格式、Clippy、Story 测试、WASM、注册表及静态站检查通过。站点覆盖 209 个双语组件和 859 个静态地址。用 `kui add holo-card` 从注册 JSON 和本地源码独立安装，首段示例通过原生及 WASM 编译。控件源码 SHA-256 `a3c370bd652f5bb6691755ac93bf6b91658666bf39460e79db30f17ef3ccc5ae`，注册包 SHA-256 `168d458b9aae9dff831c831d474822178bb866dbe4cc3d394c291f73dfd45e9f`。
- 正反面的底色、字形及光带与官网有差异。KUI 使用 GPUI 的渐变和矢量绘制近似混合模式。普通子元素在投影包围盒内排版，绘制型卡面支持逐点透视。官网弹簧系数、逐帧同速对比、Safari、原生 GPU 窗口、真机传感器、真机触摸、读屏及帧时间待核验。

### Control center · 2026-11-09

- 公开 Markdown SHA-256 `f4fca88861f0a36448b105787d77667e81d865895b1dff84cd7afd1a04732514`。公开注册包返回 404，Code 面板要求 Pro。已核对 `focusModes`、`channels`、可选联系人、音量与字号默认值、结束时间格式和整份状态通知。KUI 用 `ControlCenterChanged(ControlCenterState)` 报告提交结果和拖动时每个不同档位，`with_format_time` 接受本地结束时间。默认音量 60%、字号 100%，示例音量设为 64%。实际通知与演示策略由宿主应用。[官网预览](https://uiarc.dev/components/control-center) · [公开文档](https://uiarc.dev/components/control-center/markdown)
- 官网实测面板 360×396px，专注卡片 338×84px，下方两张卡片 165×104px，间距 8px；滑轨 326×30px，各 28 格，格宽 8.75px、间距 3px。详情 342×378px，拨盘 164px，窄宽缩到 150px，开关 40×24px，按钮高 38px。字号样例以 14px 为基准按 85–130% 缩放。官网原矩形外壳/内卡使用约 30px/22px，KUI 使用 8px Token，保留圆形与胶囊。[官网静止](/private/tmp/kui-center-ref-rest.jpg) · [专注详情](/private/tmp/kui-center-ref-focus.jpg) · [通知详情](/private/tmp/kui-center-ref-notifications.jpg)
- 官网页面实测三种专注预设 50/30/90 分钟，会议预设翻页增加 15 分钟，End 到 120 分钟。Esc 回到详情箭头且草稿未提交。总开关暂停通知时保留频道选择；演示开启的说明为 `Previews hidden`。本地 GPUI 专项测试覆盖提交/取消、到期只清除一次、拖动保持抓取偏移、竖向触摸避免误改、拨盘跨圈和取消、窄宽减少动效的几何与淡入、开关和键盘焦点回环。
- 最终 Web 预览 `6ed28153da2fc5c6fbe8`。在 1280×720 英文暗色中，从专注箭头进入，选择会议并按 Page Up，从 30 到 45 分钟；焦点内描边保留数字对比度。开始会话后，收回中间帧保留卡片标题。在通知详情关闭总开关、开启部署提醒，其余频道仍开启。Esc 后 Enter 可重新展开同一详情。音量在偏离当前刻度的位置按住后向右拖 32px，从 64% 到 74%。字号 End 到 130%，Aa 随之放大。320×720 中文亮色中，约 264px 宽面板、150px 拨盘和开始按钮可完整显示。[静止](/private/tmp/kui-center-local-rest.jpg) · [展开中间帧](/private/tmp/kui-center-local-focus-mid.jpg) · [拨盘焦点](/private/tmp/kui-center-local-draft.jpg) · [收回中间帧](/private/tmp/kui-center-local-close-mid.jpg) · [通知暂停](/private/tmp/kui-center-local-prefs.jpg) · [音量与字号](/private/tmp/kui-center-local-meters.jpg) · [320px 中文](/private/tmp/kui-center-local-320.jpg)
- GPUI 专项 4/4。格式、Clippy、故事测试、WASM、预览源校验、双语静态站检查通过。站点覆盖 208 个双语组件和 855 个静态地址。用 `kui add control-center` 分别安装注册 JSON 和本地源码，首段用法通过无窗口原生及 wasm32 编译。控件源码 SHA-256 `3285b1eed4a74467741d6e16250067955a8c9fddfffdf6688ab757cd39328b36`，注册包 SHA-256 `87ce9168202f764e3c904f75cf8aba75c86147904b92e5a004455a1bf12ee10c`。控件直接引用现有锁文件中的 chrono 0.4.45；本机缺少 `cargo audit` 子命令。
- 本轮保留 KUI 图标路径及矩形圆角。卡片图标和文字用交叉淡入淡出，参考的模糊叠加和共享元素连续曲线仍待同速核对。Safari、原生 GPU、真机触屏、屏幕阅读器与 60Hz 帧预算待实测。上轮原生宿主检查在本机缺少 Metal Toolchain 时停在 GPUI Apple 依赖；无窗口原生测试和独立原生安装编译另计。

### Wallet stack · 2026-11-09

- 公开 Markdown SHA-256 `fd13068049030ef93ca2cce7797ebbcfb5879694a814023b25922608f2a30c81`。公开注册包返回 404，Code 需要 Pro 权限。核对卡片顺序、四种材质、旧 finish 映射、尾号、有效期、网络标记、图片、余额、交易、语言/货币、选中通知和样式扩展。KUI 原创 Rust 实现保留 `USD`、`en-US` 和 `Metal` 默认值，以 `WalletStackSelected(Option<WalletStackCard>)` 映射展开和收回的通知。金额格式有宿主覆盖接口。[官网预览](https://uiarc.dev/components/wallet-stack) · [公开文档](https://uiarc.dev/components/wallet-stack/markdown)
- 官网实测卡片宽 296px，ID-1 比例计算高度约 186.7px。选中后缩至 88%，未选卡收到底部约 30px 的可见区域。预览实测约 542px 高，卡堆标题下方保留固定位置。KUI 同步卡片尺寸、余额横排、交易顺序和演示内容，矩形卡面采用 KUI 8px 圆角。官网各弹簧系数与内层材质源码未公开。[官网静止](/private/tmp/kui-wallet-ref-rest.jpg) · [官网展开](/private/tmp/kui-wallet-ref-open.jpg) · [官网黑卡](/private/tmp/kui-wallet-ref-black.jpg) · [官网扇开](/private/tmp/kui-wallet-ref-fan.jpg)
- GPUI 专项 4/4 覆盖闭合 Tab 顺序、Enter 展开、打开时换卡、Esc 恢复焦点、连续拖动、一次最多换一张、下拉收起、触屏取消和减少动效。中间帧复现了指针高光误取黑卡贴图。新增回归测试复现旧缓存键冲突，再用扁平材质键拆分缓存；高光的透明度由悬停进度控制。修正亮色阴影横向被切断，账目显现跟随打开和选中卡片位置。
- Web 预览 `41a4f34db3decaa888d9`。1280×720 英文暗色点击银卡后拍到余额递增和交易依次出现，中间帧保持银色。打开时按 Right 切到黑卡，Esc 收起后向下拖动 113px 进入第 3 张，玻璃卡与背后卡面文字分离。打开第 4 张核对预付金额和三笔交易。320×720 中文亮色预览内测到卡片收拢为 264px 宽，三条交易和收回按钮可完整显示。[静止](/private/tmp/kui-wallet-local-rest.jpg) · [中间帧](/private/tmp/kui-wallet-local-open-mid.jpg) · [展开](/private/tmp/kui-wallet-local-open.jpg) · [黑卡](/private/tmp/kui-wallet-local-black.jpg) · [拖动后玻璃卡](/private/tmp/kui-wallet-local-drag.jpg) · [预付卡](/private/tmp/kui-wallet-local-color.jpg) · [320px 中文](/private/tmp/kui-wallet-local-320.jpg)
- 格式、Clippy、故事测试、WASM、预览源校验和静态站检查通过。站点覆盖 207 个双语组件及 851 个静态地址。从注册 JSON 和本地源码分别用 `kui add wallet-stack` 安装，首段文档示例均通过原生和 wasm32 编译。控件源码 SHA-256 `844ec0fe4f78ecb2436b8908026825a24a4f640257bf0c65cc58b2f0582e4f91`，注册包 SHA-256 `857a8ebdac12cfa9ad28b1e34cc0fc3553a2bf5ccb42c901c64709e426ff1027`。macOS 桌面宿主的 `cargo check -p kui-story --features native` 在 GPUI Apple 依赖编译着色器时遇到本机缺少 Metal Toolchain，桌面 GPU 验证待补。
- 本轮保留不同的卡面纹理、字形、矢量芯片与虚构网络标记。透视用轴向投影近似，玻璃卡为半透明材质。设备光照由宿主传入。金额格式内置支持一组地区和币种。设备倾斜、真机触摸与外层横向手势、Safari、原生 GPU 实拍、读屏播报、同速逐帧和帧预算待验。

### Action morph · 2026-11-09

- 公开 Markdown SHA-256 `9430a550ffe944b8dbcd4f8922633bf71adaf4189f7975dab1025881c1ceafeb`。公开注册包 404，Code 面板提示需 Pro 权限。核对了动作、菜单说明和快捷键、单/多行表单、单选组、空输入提示、起始/末端锚点及 1600ms 复位。KUI 用带编号的 `ActionMorphSubmit` 和 `complete` 映射同步或异步保存。[官网预览](https://uiarc.dev/components/action-morph) · [公开文档](https://uiarc.dev/components/action-morph/markdown)
- 官网实测浮动按钮 56px，菜单 256×220px，三个菜单行高约 55.4px、间距 2px。任务和提醒表单 320×221px，便笺 320×196px。单行输入约 42.4px，多行高 88px，选项轨道与提交键均为 36px。闭合为圆形，展开面板官网圆角 24px，KUI 使用矩形面板 8px。实测失败后保留草稿与日期，切换动作后可重试并仅新增一项。[菜单参考](/private/tmp/kui-action-morph-ref-menu.jpg) · [失败参考](/private/tmp/kui-action-morph-ref-failure.jpg)
- 同一 GPUI 表面固定底角，宽高和圆角由弹簧驱动，进入与退出的面板错时滑动和淡入淡出。保存期间防止重复提交，失败恢复编辑焦点，成功仅清除对应草稿并回到创建按钮。三项专项测试覆盖 65ms 中途反向、窄宽与减少动效、方向键及动作快捷键、便笺换行与组合键提交、空输入、失败恢复、旧回执拦截和 1600ms 复位。
- Web 最终预览版本 `2707da9c2b3b4011fb0c`。浏览器走查了空任务报错、失败保留 Tomorrow 和文字、便笺换行、切回原草稿、重试只新增一条任务。最终复验初始高亮在首项、提醒默认 5:00 PM、回车图标正常，以及 320px 亮色中文表单宽 256px。失败后原中文文字、选项与提交键均完整显示。[本地菜单](/private/tmp/kui-action-morph-local-menu.jpg) · [本地提醒](/private/tmp/kui-action-morph-local-reminder.jpg) · [任务重试成功](/private/tmp/kui-action-morph-local-saved.jpg) · [320px 中文失败](/private/tmp/kui-action-morph-320.jpg)
- 从注册 JSON 与本地源码分别安装到空白项目，首段文档用法通过原生及 WASM 编译。注册包 SHA-256 `2897937c3261e99fb7120e6939c2be87ba01485336d00bee92232556e64cd031`，控件源码 SHA-256 `240e4a75bb2030eea7041265cf619436349c47d2052f4a43dc4f522391710576`。格式、Clippy、故事测试、WASM、预览源校验和站点检查通过，覆盖 206 个双语组件和 847 个静态地址。
- 当前图标为 KUI 矢量图，矩形圆角、字体和弹簧曲线与官网仍有差异。错误说明和保存状态有 GPUI 角色或描述，浏览器表单的 `aria-invalid`、`aria-busy` 与实际读屏尚未对齐。官网同速逐帧、Safari、原生实拍、真机触摸和帧时间待测。

### Glass tab bar · 2026-11-09

- 实际 ID 为 `glass-tabbar`。公开 Markdown SHA-256 `9f8766a1cd28e80ff537c02b3beaf876f3a012a81b86f65be8aba5c80493a980`，公开注册包 404，Code 面板提示 Pro 授权。已核对 items、标签、受控/非受控值和收起状态、变化事件、独立操作、滚动绑定与布局扩展；对应的 Rust API 支持 SVG 或 GPUI 矢量图标，以及数值和文本徽标。[官网预览](https://uiarc.dev/components/glass-tabbar) · [公开文档](https://uiarc.dev/components/glass-tabbar/markdown)
- 官网桌面实测 420×560px 滚动区，底部内缩 22px，主轨道 320×62px，槽宽 78px，镜片高 54px，操作泡 62px，间距 10px，图标 22px，标签字号 12px/16px。下滚后主轨道 200×48px，槽宽和操作泡均为 48px，镜片高 40px。移动预览内四个展开槽为 59px；KUI 窄屏进一步压缩槽宽以保持整栏可用。实测搜索参与方向键回绕，End 选中搜索时淡出主镜片。[展开参考](/private/tmp/kui-glass-tabbar-reference-open.jpg) · [收起参考](/private/tmp/kui-glass-tabbar-reference.jpg)
- 亮暗材质实测 alpha 分别为 0.56/0.58，背景模糊 18px，饱和度分别为 1.8/1.6。同步亮暗描边、阴影和标签颜色。KUI 的展缩和镜片使用 400ms 平滑弹簧，速度参与抬升、拉伸和图标放大。[公开动效](https://uiarc.dev/docs/motion)
- 三项 GPUI 专项测试覆盖受控拖动只提交一次、65ms 中途反切连续、搜索回绕、Enter 恢复展开、320px 级布局、竖向触摸滚动和减少动效。缩放和指针命中使用同一份布局几何。
- 最终 Web 版本 `061d9f075cf16d6aa913`。在本地英文预览内下滚，外层正文保持在 281px，栏位收成 48px。将首页镜片拖到收藏后落在收藏。End 选中搜索并淡出主镜片，Right 回到首页，Enter 展开。亮暗实拍及 320px 中文实拍显示四个标签、收藏徽标和独立搜索完整。预览加载阶段有一条来源待定位的 MutationObserver 日志。已搜索站点和预览的生成脚本，该日志未阻断上述实测操作。[暗色展开](/private/tmp/kui-glass-tabbar-dark.jpg) · [暗色收起](/private/tmp/kui-glass-tabbar-compact.jpg) · [亮色展开](/private/tmp/kui-glass-tabbar-light.jpg) · [320px 中文](/private/tmp/kui-glass-tabbar-320.jpg)
- 独立项目分别从生成注册 JSON 和本地源码安装，文档 Rust 示例通过原生及 WASM 编译。注册包 SHA-256 `361ce3df792084960e5642319415d6e50ad098f4a2937e10257bc57e755b6457`，控件源码 SHA-256 `b4da7765c715f19068367da64ae853362e569666e89fd5f94e001d2b194fb1e2`。格式、Clippy、故事测试、WASM、注册表和站点检查通过，验证了 205 个双语组件和 843 个静态地址。
- 仍有差异。KUI 使用原创滚动场景和宿主提供的背景纹理重采样，官网 Chromium 使用实时像素位移及色散。当前竖向触摸绑定到 GPUI 滚动区域，惯性未对齐；减少透明度与高对比由宿主设置。官网同速逐帧、Safari、原生实拍、真机触摸、读屏和帧时间待测。

### Orbit menu · 2026-11-09

- 公开 Markdown SHA-256 `e81211e3905d91e058ca0a94fdcffbf6591aec37935bf2e6513dd82aa9d37913`。公开注册包返回 404，Pro 源码未取得。公开默认参数为半径 104px、展开角 150°、方向 −90°、`disabled=false`、`announce=true`，手势死区 34px。[官网预览](https://uiarc.dev/components/orbit-menu) · [公开文档](https://uiarc.dev/components/orbit-menu/markdown)
- 官网实物测得 56px 触发键、展开比例 0.94，五个 46px 圆形动作由 −165° 至 −15° 排布。选中镜片为 50.6px，轨道描边 1.5px，标签胶囊高 32px，居中放在展开方向半径的一半处。桌面画板 522×440px、图片高 200px，窄版画板高 420px、图片高 170px。照片在展开时缩放到 0.97 并淡到 0.55。[官网默认实拍](/private/tmp/kui-orbit-reference-closed.jpg) · [官网展开实拍](/private/tmp/kui-orbit-reference-open.jpg)
- 官网实测 330ms 按住不移动仍保持打开，345ms 会收起。拖向左端点赞后计数从 128 变 129，向下松手及同方向超出约 164px 范围的松手都未触发。Arrow Up 从末项开始，Esc 关菜单。KUI 按实测区间取 340ms 分界，回执用独立 1.4 秒计时器，松手才发动作事件。
- `OrbitAction`、`OrbitTone`、`OrbitInvoked`、`OrbitOpenChanged` 映射公开接口。宿主处理动作，示例点赞和相册可撤回，删除可重置，分享为演示记录，下载保存已打包图片。KUI 保留圆形控件，示例卡片和画板使用 8px。照片使用仓库已有 NASA 档案素材。[照片来源](https://images.nasa.gov/details/as17-147-22465)
- GPUI 四个专项测试覆盖 339/340ms 分界、死区及弧外取消、一次触发、1399/1400ms 回执边界、键盘回绕与 Tab 离开、取消触摸及旋转角度。半透明材质已接入，官网背景模糊、错峰与磁吸精确时序、同速逐帧、Safari、原生实拍、真机触摸及帧时间仍待验收。
- Web 最终预览版本 `48bace552d8ff415c4a1`。实测短按展开、Arrow Up 高亮末项、Esc 关闭、拖动点赞使计数 128→129、弧外松手保持计数、键盘添加相册。从画布内滚轮使正文滚动 216px。320px 浅色中文内嵌宽 288px，动作全部可见，直接点左端圆钮也能点赞。[暗色展开实拍](/private/tmp/kui-orbit-final-open.jpg) · [浅色 320px 实拍](/private/tmp/kui-orbit-mobile-light.jpg)
- 下载适配层验证生成 19,722 字节的 `image/jpeg` Blob、预期文件名与对象 URL 释放。Blob 的 SHA-256 `e8467328289cd499fac6c10a6bb2b74533cee2cb18b9ca8dda36dfe88592159c` 与打包素材一致。Codex 浏览器未收到下载事件，本次没有确认文件落盘。
- 从注册 JSON 及本地源码分别 `kui add orbit-menu` 安装至空白项目，文档首段用法通过原生和 WASM 编译。注册包 SHA-256 `ec17388f9ac24a71ebb72d9d94761a94335401c6891c3cde9fa5de539bfba524`，控件源码 SHA-256 `8cba5b9cc493360b1c213ca0f5bbfa02cd76bae63cb26a63c1506419fde9a084`。格式、Clippy、故事测试、WASM 和 `verify-preview` 通过，站点检查通过 203 项双语组件和 835 个静态地址。

### Liquid tab bar · 2026-11-09

- 公开 Markdown SHA-256 `ebdc750f06acf195472c1c756f4e60e29d55fa048ea32d2431e6aeb3ce71f07b`。公开注册包返回 404，Pro 源码未取得。公开动效规范 SHA-256 `99d8750bddc1545e3ea28fb8432662d941ed5ce5a753e973986a00ac33e48fdc`，`morph` 为 420ms、`bounce` 为 0.16。[官网预览](https://uiarc.dev/components/liquid-tab-bar) · [公开文档](https://uiarc.dev/components/liquid-tab-bar/markdown) · [动效预设](https://uiarc.dev/r/skills/arc/motion.md)
- 实物测得轨道高 48px，镜片及槽高 40px，基础槽宽 44px，外内缩 4px，槽间距 2px，图标 20px / 1.75 描边（24 单位网格）。默认四槽和 Home 标签共 239px，仅图标 190px，全部标签 394px。标签为 14px / 500，英文字符间距 −0.14px，普通文字起于槽内 x=40，单数字徽标让出额外 6px，徽标 17×16px。键盘环在槽外 2px，线宽 2px。官网移动预览只展示当前标签和仅图标两个模式。
- Rust 参数映射覆盖 `tabs`、`value`、`LiquidTabChanged`、三种 `LiquidTabLabels`、生成或指定的 `id` 与标签默认值。`LiquidTabItem` 支持 `Glyph`、专用矢量图标和单色 SVG 资源，可单独指定选中图标。徽标使用数字及可选读屏名称。宿主回写选择并呈现面板，`liquid_tab_id` / `liquid_panel_id` 提供稳定标识。GPUI 暂无这个控件所需的原生 tab→panel 关联属性，平台读屏关联仍待验证。
- GPUI 绘制保持一套真实焦点目标。镜片按轮廓裁切反色图标、文字和滚动数字。正常静止镜片高度固定，拖动时小幅收拢。左右键回绕，Home/End 切两端。四个专项测试覆盖键盘与三种槽宽、甩动一次提交和 65ms 反切连续、竖向与取消触摸、减少动效及徽标更新。Web 实测三种模式、收件箱局部反色、拖回首页及 End 切到个人页。[官网暗色实拍](/private/tmp/kui-liquid-reference-dark.jpg) · [本地默认实拍](/private/tmp/kui-liquid-kui-dark.jpg) · [本地全部标签实拍](/private/tmp/kui-liquid-all-dark-full.jpg)
- 最终预览 `a5ef9fa80881b4150a95`。同为 522×260 的暗色默认截图中，以亮色阈值量镜片，中央横向像素均为 148–235，纵向官网 60–98、本地 60–99。320px 中文页内嵌宽 288px，四个槽位及收件箱未读徽标完整，悬停镜片滚轮时正文滚动 230.5px。键盘外环在暗色版实拍可见。[键盘焦点实拍](/private/tmp/kui-liquid-focus-dark.jpg) · [320px 浅色实拍](/private/tmp/kui-liquid-mobile-light.jpg)
- 独立项目的 `kui add liquid-tab-bar` 从注册 JSON 及本地源码安装，文档示例通过原生和 WASM 编译。注册包 SHA-256 `eb01e8ea7647cbc4b82e2f8dec5812e87ebeab7cac5e159220a79f5714d63630`，控件源码 SHA-256 `427d519f8686b115980dda6c713ba929f0dcc5536ce7547baff649c15cf38e0f`。格式、Clippy、故事测试、WASM 与 `verify-preview` 通过，站点检查通过 202 项双语组件和 831 个静态地址。
- 镜片使用公开弹簧参数，边缘裁切用 1px 绘制条带。图标路径为原创，字体使用 KUI 的字体及字距，悬停位移、拖动速度门槛和面板时长为本地实现值。官网同速连续帧、完整静态误差、Safari、原生实拍、真机触摸及帧时间仍待补证。

### Dock · 2026-11-09

- 公开 Markdown SHA-256 `dd42797b0655e75c864535823f90ec96ad5ac6041be7aac161194ff36f10f709`，公开注册包返回 404。Pro 源码未取得。官网实测六个 40×40px 固定槽位、4px 间隙、6px 内边距，栏体 272×52px。四个 36px 形状工具组成 168×48px 托盘，从上方长出并与栏体共用轮廓。选中底片和标签在槽位之间滑移。[官网预览](https://uiarc.dev/components/dock) · [公开文档](https://uiarc.dev/components/dock/markdown)
- Rust 映射保留 `items`、默认空选中的 `value`、`DockSelected` 选择事件、`DockReordered` 受控排序和 `with_label`。快捷键提示由组件显示，实际快捷键由宿主绑定。`with_reorderable` 默认关闭，与参考未接收排序回调时一致。图标用 `DockIcon`，外层布局交给宿主 GPUI 容器。任意自定义图标与多级子项仍需审查。
- 五个 GPUI 专项测试覆盖静止槽位与托盘尺寸、方向键选形状、Esc 焦点返回、拖动和 Alt+Arrow 排序、纵向触控手势不改序、拒绝回写保持宿主值，以及点击托盘时隔离下方画板。Codex 浏览器实测选椭圆后只落一笔、移动键换位后保持选中形状，解决一条批注后徽标从 2 变 1。
- 官网落笔实测矩形 96×64、圆形 76×76、三角形 84×74、星形 80×77，描边 1.5px。KUI 将画板形状与工具小图标分开绘制，描边与填色按官网亮暗两组实测颜色换算为 sRGB。KUI 矩形使用 8px 圆角，照片使用仓库现有 CC0 素材，三角和星形的圆角曲线用原创近似。落笔淡入缩放的 200ms 为 KUI 当前实现。
- 最终预览 `dd6712d9b90f7d65b9a0` 实测键盘选形状、快捷键切换、四种落笔和亮暗主题。320px 中文页内嵌宽 288px，栏体六个槽位、展开托盘四个工具和“矩形”键盘提示完整显示。站点验证 201 个双语组件和 827 个静态地址。格式、Clippy、故事测试、WASM 检查与 `verify-preview` 通过。[官网默认实拍](/private/tmp/kui-dock-arc-default.jpg) · [本地暗色实拍](/private/tmp/kui-dock-final-shapes.jpg) · [本地 320px 实拍](/private/tmp/kui-dock-mobile-light.jpg)
- `kui add dock` 分别从注册 JSON 和本地源码安装至空白项目，文档首段 Rust 用法通过原生与 WASM 编译。注册包 SHA-256 `dabb55688b39d784c641f50e45da867ba7e843712cc52b62c1dfef9cfd7a382b`，共享控件 SHA-256 `97e7d0f8684015e667ece9b7063fd25a7116b100fcc88d7ed28732112ed6e518`。
- 圆角按 KUI 规范使用 8px，群组图标短模糊与托盘背景模糊还未等效。标签悬停延时、弹簧中间帧、Safari、原生窗口实拍、读屏、真实触屏和帧时间继续验收。

### Morph nav · 2026-11-09

- 公开 Markdown SHA-256 `eca5461c363abb75f283e8b82b9e40e9f7b7e90c2695a7a15382263562e09d8b`，公开注册包返回 404。参考官网实物和公开参数，重做原未导出的 Rust 模块。实测默认栏高 56px、按钮高 36px，紧凑栏 46px、按钮 32px，品牌名收成图标。窄容器的 Menu 展开分组和链接，宽栏 Product 为两列链接、特色卡和底部入口。[官网参考](https://uiarc.dev/components/morph-nav)
- 对照默认 720px 容器断点、420px 视口单列、空查询五条建议、筛选最多六条、悬停 60ms 展开和移出 180ms 关闭。宿主用稳定 ID、可选 href 与父栏目处理导航，搜索用独立索引。注册特性卡和页脚支持宿主链接，特色图可传 GPUI View。KUI 演示使用原生绘制的特色图与 8px 圆角。
- 四个 GPUI 专项测试通过，覆盖键盘跨组、窄栏选链、搜索限额和独立关键词、焦点归还，以及 59ms/60ms 悬停门槛、移出 179ms 时仍展开、关闭后高度连续收缩。命中读取绘制后的栏体范围。Codex 浏览器已走通窄栏分组、搜索 Slack 返回集成并提交、宽栏按向下键展开双列、特色图及页脚。
- 最终 WASM 版本 `3a2fcd40f11ed4f5d8a5` 复拍闭合和展开时的顶栏，上缘都在预览内约 26px。320px 视口下 iframe 宽 288px，菜单改为单列，顶栏四个控件可见。桌面宽屏的预览可跨过组件的 720px 断点。双语站点检查通过 200 个组件和 823 个静态地址。
- 双路径 `kui add morph-nav` 在空白项目通过原生与 WASM 编译。注册包 SHA-256 `d2e5e22b0ba3684b97e2e678100a9d931907aaa60e83d583c3899d1cb6c0efd2`，Rust 控件 SHA-256 `46008a0ce47f3be9d2e1a7ad572b348407ba1f7003bd0006bea45f500d9fc551`。
- GPUI 采用半透明表面，参考中的背景模糊和搜索模糊尚未等效。展开回弹、连续换组逐帧、真实触屏、Safari、读屏属性与帧时间继续验收。当前机器 Metal Toolchain 缺失，真实桌面窗口仍待验证。

### Trial banner · 2026-11-09

- 公开 Markdown SHA-256 `a2732252184f14ebf9f9a439ba9d20341ef09afc45adf060a40fba033de37c69`；公开安装包返回 404。以公开 API、官网预览与浏览器布局测量为依据，用 Rust 重建。官网实测 521.6px 容器内环 28px、按钮 36px；侧栏卡 340×411px、环 40px、主按钮 44px；警告胶囊 36px 高。KUI 矩形遵守 8px 设计规则，胶囊和环保留圆形。[官网参考](https://uiarc.dev/components/trial-banner)
- 官网预览剩 9 天时有明天、3 天后、剩 3 天提醒三个选项；剩 2 天或已结束时只有明天。结束态保留提醒入口，主操作改为升级。KUI 由宿主传入日期，用 `TrialEvent::Snoozed` 返回所选日期。实际账单由宿主任务接入，预览只产生模拟结果。
- GPUI 专项测试验证剩余天数和提醒日期、重复付款防护及失败重试、键盘提醒选择与焦点恢复。宽度 710/522/320px 下核对按钮换行，侧栏卡样例包围盒为 340×411px。
- `kui add trial-banner` 分别从注册 JSON 和本地源码安装到独立项目，安装源码及中英文代码页相同的 Rust 示例均通过原生与 WASM 编译。注册包 SHA-256 `7bad79d3da200f4d61af53eea23673051550f47224ec28ca6b9bd8cdccdb28a8`，控件源码 SHA-256 `cf8b6ea9077ce1a18af299bb3a8a4bf9c58f81e7e4e2422ca03d3ce90b424346`。
- Codex 浏览器实测中文侧栏卡、两天警告、明天提醒、折叠及重新打开。最终 WASM 预览 `45f76828cc94ab0d0da1` 中，弹层打开后 iframe 从 520px 增至 574px，卡片贴着触发键下缘。按 Enter 完成模拟升级，回执留在浮层内。已复查窄行文案的句号。站点验证 199 项双语组件、819 个静态地址，`verify-preview` 验证 237 个文件。
- 官网付款按钮的实测请求被自动审批拦截，理由是可能启动付款流程。官网加载、失败和升级态以公开说明为依据。需要补同场景静态对照、颜色和宽度逐帧动效、Safari、真触摸、读屏以及长期帧预算证据。当前机器 `xcrun metal --version` 返回缺少 Metal Toolchain，原生窗口实拍仍待补。

### 新接入 AI mascot、Install snippet · 2026-11-09

- 分发验证通过。AI mascot 与 Install snippet 各在两个空白项目中分别从注册 JSON 和本地源码安装，文档示例均通过原生及 WASM 编译。站点检查通过 195 个双语组件和 803 个静态地址。浏览器手动切到 Done 后，大角色与 24px 行内角色同步显示完成状态，自动轮播停止。安装片段实测切换到 KUI CLI 命令，再生成样例密钥，等待状态结束后第二步打勾。两项只记公开参考和分发通过，完整视觉、动效和交互继续待验。
- 桑基图补验了窄屏标签。官网独立预览在图幅 216px 时，Pro 和 Team 标签分别从相对 x=150.20 和 x=202.20 起排。KUI 取消直立模式对标签横向锚点的强制回推，本地 320px 页面复拍确认两个读数分开，末列仍完整。此项复测不覆盖任意宿主长标签、触摸或同速逐帧。

- AI mascot 以公开预览的六个表情、148px 主示例、24px 行内例和 420ms 状态融合为参考。KUI 使用原创矢量曲线，保留 96px 默认、24–160px 范围、56px 紧凑阈值、可关闭伴随点、宿主状态和鼠标范围设置。公开版未提供 Pro 源码。眼部弹簧、精确眨眼和轨道时序、读屏主动播报仍待逐项核对。[官网预览](https://uiarc.dev/components/ai-mascot)
- Install snippet 按公开预览实现等宽框架标签、三步布局、块内横向滚动、剪贴板反馈和 240ms 框架高亮。KUI 示例用 Rust/GPUI 接入文案，密钥仅为本地合成样例。测试验证取消后丢弃迟到结果、失败重试和确认回执。包管理器高亮滑动、跨片段淡入淡出、密钥 340ms 揭示、窄屏标识仍待实现或对照。[官网预览](https://uiarc.dev/components/install-snippet)

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

> 阅读 AGENTS.md 和本文件。原108项按用户要求排除，只处理表中指定的新组件。先查该组件行、docs/items规格、registry真实源码和公开官方参考，复现样式/布局/输入/连续动效差异，填写S/A/V/I/M/P/D/F证据。只有本地能渲染或编译成功不能判定对齐。Activity rings和Sankey flow优先；其他Agent正在修改的文件先避免覆盖，记录版本再协调。当前按用户要求同步实现和审查。每完成一个阶段同步该行与桌面镜像，不批量勾选通过。

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
