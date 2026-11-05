# 组件复核记录 · 2026-11-03

> 历史证据存档。2026-11-05 用户要求新增85项全部重新审查；当前结论以[新增组件重审清单](new-components-re-audit-2026-11-05.md)为准，本文件不作为新一轮通过证明。

这份记录区分公开源码值、公开预览测量和 KUI 平台修复。它不是全站逐帧验收通过声明。

## 已核对的公开源码参数

| 组件 | 源码依据 | 参数与本轮处理 |
| --- | --- | --- |
| AnimatedCounter | [公开注册表](https://uiarc.dev/r/animated-counter.json) | 更新弹簧 `smooth` 为 400ms；首次揭示为 480ms，逐列延迟 `min(order × 35ms, 250ms)`；首次揭示阈值 60%；默认不开启首次揭示。数字沿整体数值增减方向滚动，包括负数。字形透明度为 `max(0, 1 - abs(offset))`。独立计数器行高为 1，边缘窗口延伸 0.16em，字重 500。 |
| ToggleGroup | [公开注册表](https://uiarc.dev/r/toggle-group.json) | 默认中号段高 36px，轨道内边距 3px，段间距 2px，带文字段水平留白 10px；单选底片使用 420ms 的 `morph` 弹簧。 |
| BillingToggle | [公开注册表](https://uiarc.dev/r/billing-toggle.json) | 中号段高 34px，轨道内边距 3px，段水平留白 14px，带徽标段右侧 5px；徽标高 24px、水平留白 8px。底片 340ms，文字进入 240ms、退出 160ms、位移 0.45em。预留最长候选文案宽度；只有显式传入 active 文案才换字，不自动改写优惠文案。 |
| 共用动效 | [公开 Token](https://uiarc.dev/r/arc-motion-tokens.json) | `smooth=400ms`、`morph=420ms`、`considered=480ms`、`standard=240ms`、`fast=160ms`、`stagger.item=35ms`。按动作使用各自参数，不统一替换为一个时长。 |

上述时间是源码的 visualDuration 或 duration，不能解释为弹簧必须在该毫秒数完全静止。

## 公开预览对照

SplitBar 的 [公开文档与预览](https://uiarc.dev/components/split-bar) 可访问，完整实现源码未取得。2026-11-03 通过浏览器读取公开预览的计算样式：柱高 44px；标题 16px/500；说明 14px；柱内和外侧标注 12px；图例 14px；主要读数 18px/500；图表块间距 12px；标题与说明间距 2px；外侧标注名称与数字间距 4px；数字行高 1.2、窗口延伸 0.18em。上述属于公开渲染值，不标为 Pro 源码值。

本轮修正右边界标签聚集、窄屏标签互相覆盖、隐藏首尾分段后圆角归属及数值滚动。标签过密时保留百分比，完整名称仍在图例；极窄容器必要时增加标注行。最后一项属于 KUI 的溢出处理，不声称已与未公开源码一致。

圆角遵循用户明确指定的 KUI 8px，覆盖参考控件原来的更大圆角。Rust 接口继续使用 KUI 类型和已有安装 ID。

## 平台与负载修复

- `vendor/gpui-pre-web` 保持 0.3.10。物理像素盒与 CSS 尺寸乘 DPR 不一致时采用 CSS 回退，避免模拟窄屏的布局翻倍和命中错位；一致的像素盒和 Safari 回退保留原行为。
- 共用控件在宿主隐藏时停止请求动画帧；恢复可见时请求一次重绘。Funnel 粒子保留相位，隐藏时不累计跳变。
- 编译和测试串行执行，并限制负载。只保留本地静态服务，不需要常驻测试或浏览器录制。

## 验证与边界

启动期网络导入失败现在只自动重试一次，已用隔离的缺失资源页面验证：仅发生两次文档请求（首次加一次重试），持续失败后停止；恢复资源后可正常绘制。运行中的控件不会因此重启。

在临时源码快照中完成标准 Clippy、259 项 Rust/story/集成与文档测试、5 个像素比例回归场景、优化版 WASM 构建、注册表一致性与 193 项双语文档的 795 个静态目标检查。本轮目标控件源码与快照逐文件比对；不把其他 Agent 此后的修改算入这次验证。

实际画面检查包括 Safari 的亮暗主题和窗口缩放，以及 Codex 浏览器的窄屏分段图表与预览内滚动。并未完成全站逐帧、1px 误差或恒定 60fps 验收。

仍需单独验收：CSS 字形模糊与羽化遮罩的 GPUI 等价效果、ToggleGroup 按压细节、所有组件的快速反转中间帧、未公开 Pro 源码参数。基础 CSS Token 未取得源码值时不从截图反推。不得据此声称每个组件的每个参数已经完全一致。

## 下载证据指纹

公开注册表下载内容的 SHA-256；用于区分后续官网更新。

| 来源 | SHA-256 |
| --- | --- |
| `animated-counter` | `01046ee4db66aa33671f460ee76bbae54db7b0059a9ccde4b77da4548ac5453a` |
| `billing-toggle` | `3ff007b6378dafb7525f6ea7b5ce9e70271225c3b614d957cf25787e9037b901` |
| `toggle-group` | `9eb1f976313144be96811ca6739cf8adfe002c94eddcec90e811d2e2cec4122f` |
| `money-input` | `321456ad3da3c0e569143085f52d8ef6d4c1df2c3ea50d219e157a357bbd2a9a` |
| `scrub-input` | `ba705613dbc373d60c7ce762cc98f9721a9a46fd9de3f5cb8114e265ca68238f` |
| `motion-tokens` | `2adf39247da5320283f23c446cea7976572c693bcadf9dc7d9c4e7e4e217b141` |

## MorphLoader：公开预览脚本复核

取得的是官网浏览器公开加载的 [专属预览脚本](https://uiarc.dev/_next/static/chunks/93677.b77ebef2ae53b0e4.js)，不是付费源码压缩包或原始 TypeScript。SHA-256：`4c7170b354307e0470fa61978a6e2bf479740a24a9ccb988cb80ae1227793a72`。下列常量可直接在该脚本中核对；Rust 绘制与运动模型为本仓库实现，没有把脚本加入安装包。

| 范围 | 已核对参数 |
| --- | --- |
| 默认 API | dots / loading；size 24；strokeWidth 2.5；tone true；decorative false；Loading / Done / Failed。 |
| 图形 | 24 单位网格；dots 中心 x=5/12/19，宽4；bars x=4.5/9.5/14.5/19.5、y=6…18，宽3；ring 半径8.5、半角32°、曲率1/8.5；square 端点5…19。 |
| 完成几何 | success 四段依次连接 (5,12.5) → (7.6,15.1) → (10.2,17.7) → (14.6,12.6) → (19,7.5)；error 两条对角线在(12,12)分成四段，端点6.5/17.5。 |
| 弹簧 | ω=2π/(1.2×seconds)，stiffness=ω²，damping=2ω，mass=1，restDelta=.001，restSpeed=.01；shape .46s，result .52s，rotation .60s；result 每段延迟35ms。 |
| 循环 | dots 周期.9s、相位差.8、升幅3.2、增粗.6；bars 周期.95s、相位差.7、长度减幅7.5；ring 330°/s、呼吸周期1.4s、长度减幅3.2；square 周期.9s、每次90°、前80%平滑推进、长度减幅8。循环单帧 delta 最大64ms。 |
| 结果 | 循环淡出180ms、恢复240ms，standard easing(.22,1,.36,1)；朝前回正至360°倍数；scale 1→1.14→1，420ms，关键点35%，分别使用 easeOut/easeInOut。 |
| 演示 | size72、stage高220、stage/controls间距20；选项高30/水平留白12/min-width60，轨道padding2/border1；actions高36/水平留白14/间距8；控制组间距10；结果1700ms后复位并在等待时禁用两结果按钮。 |

修正直线拼接圆环、圆点和竖条的错误线宽、仅两笔的完成图形、方形未开角及演示布局。圆角仍遵循用户指定的 KUI 8px，不沿用参考演示的胶囊。Pro 其余组件未取得的实现参数仍保持未核实，不能由这一项推论全部通过。

同页公开 [样式表](https://uiarc.dev/_next/static/css/2eff092df847e747.css) 还确认：加载态继承 currentColor，结果色用 standard easing 240ms 过渡，减少动效时颜色过渡为120ms。Rust 画布在绘制上下文读取父级文字色，并在快速反切时从当前可见颜色继续过渡。几何的减少动效仍是静态即时切换。

本项验证：原生整体263项测试通过后，又补充并通过颜色中断连续性回归（本控件最终6项）；最终 Clippy、组件无默认特性 WASM 检查、Web WASM 检查、优化构建、注册表 preview=source 检查通过；193 项双语文档与795个静态目标检查通过。Codex 浏览器实测了中文/英文、亮暗主题、方向键选形状、320px换行和预览内滚动。窄屏宽度≤420px时选项取消60px最小宽度，水平留白改为10px，选项字重继承普通字重；均来自同一公开样式表。未完成同尺寸60Hz逐帧误差验收，不标记为全部视觉与动效验收通过。本轮没有提交、推送或发布。

补充连续操作复核：控件的7项回归和完整story的1项“失败→切换形状→过期复位回调”用例通过。图形与颜色共用一个待处理帧通知，并明确通知加载器实例；同一帧不重复排队。最终干净预览版本为 `76c6a852c105091dc98c`。

测试工具边界：本轮观察到浏览器局部截图后，后续画布有停留旧状态的现象；刷新后仅使用完整窗口截图，同一失败恢复流程、持续圆环与快速换形状均正常。未把该现象归因为官网动效参数或已证实的生产缺陷。后续使用完整窗口实拍，图片裁剪在保存后进行。临时诊断只在快照与本地服务中使用，已恢复，最终源码和预览不含诊断日志。仍未完成全站逐帧和60fps验收。

## AiSpark：公开预览实现逐参数核对

来源为官网浏览器实际加载的公开 [预览 JS](https://uiarc.dev/_next/static/chunks/89584.691dca91fdf346da.js)、[控件 CSS](https://uiarc.dev/_next/static/css/836431a4c5750032.css) 和 [演示 CSS](https://uiarc.dev/_next/static/css/adb2653f560425e8.css)，不是付费源码压缩包。对应 SHA-256 为 `0321d84aa2028661192f9203b6889eea350bb59d53bb1a064645f3fcb33084f9`、`345d3e78fd55ad336c242dcfa6c2495fa1d2959929bd018ffa29233a164238f3`、`87dd5dceeb897cbf3723a0c921be35736a6496f87e0bd029ea09370c5e71e4e7`。仅参数和行为用于对照，未把参考脚本复制进组件安装包。

| 范围 | 源码参数 |
| --- | --- |
| API | status未设置时自动播放；Mark/Pill/Avatar默认24/16/32px；tone=accent，speed=1，twinkle=true，paused=false，announce=true。Avatar数字尺寸的图形为round(.56×size)。Writing保留KUI名称，对应generating。 |
| 光学尺寸 | h=clamp(log(max(1,size)/12)/log8,0,1)。主射线r0=11→13.5、r1=41.5→44、根半径8.4→5.6、尖半径3.9→1.8；≥18px的对角射线r0=23.5→22、r1=32→36、根半径4.9→3.7、尖半径2.3→1.4。<18px改为r0=r1=27的圆点，半径5.6→5；中心≥28px出现，半径2.6→3.4；粒子≥40px出现。对角/中心alpha=.66→.5。 |
| 射线 | 100单位网格，45°间隔。轮廓是两个圆的凸包，位置/半径舍入2位，opacity3位、旋转3位、整体scale4位。8道光线各有length/width/offset/alpha，并非单一强度插值。 |
| Idle | 相位偏移[0,.021,-.014,.017,-.019,.012,.008,-.016]；对角6.4s逐个闪烁，脉冲宽.32；length=1+.3a，width=1+.08a，offset=a；整体1+.018×breath(t/6.4)，中心1+.1×breath(t/3.2)。 |
| Thinking | crest=breath(t/1.9-i/8+phase)^1.35；主(.7+.3c,.94+.08c,-1+2c,1)，对角(.42+.78c,.88+.16c,-1+3c,.5+.5c)；整体.965+.03×breath(t/4)，中心.7+.22×breath(t)。 |
| Writing | 周期1.04s；心跳sin²窗[0,.2]与强度.62的[.2,.42]窗相加；两道一组相差.045，对角再延迟.17；主(.7+.32a,.96+.08a,.8a,1)，对角(.55+.65a,.92+.12a,2.5a,.6+.4a)。整体.975+.035×heartbeat，中心.9+.5×heartbeat且延迟.03。 |
| 旋转与混合 | Thinking26°/s，Writing46°/s，速度追近1-exp(-3dt)；第一帧1/60，delta上限1/30。静止态朝前归到ceil((angle+.2×max(0,v))/90-1e-6)×90，以保留初速度的三次轨迹滑行.6–1.8s。状态混合Writing .42s、Done .5s、其他.56s，cubic-in-out；减少动效统一.2s。中断后原混合仍按当前循环时间求值，不冻结旧姿态。 |
| 收拢与抖动 | Done收拢.68s，前.17s easeOutCubic、后.51s cubic-in-out恢复；主/对角length减幅.12/.2、offset减2.4/3、整体减.045、中心增.35。Error x=3.2×sin(2πt/.15)×(1-t/.46)²，.46s截止。冲击记录.7s后移除，重复状态不重播。 |
| 粒子 | 半径1.9/1.3，轨道半径47.5；起始角30/210°，周期2.6/3.9s，局部scale=.5+.5×activation；opacity=activation×(.55+.45×breath(t/1.3+.5i))，fillOpacity=.8/toneB。粒子不随射线组整体旋转，只受外层缩放/抖动。 |
| 颜色与文字 | 图形/背景320ms standard，减少动效160ms；Error的Oklab混色danger72%+muted28%。Pill高28、padding7/12、gap8、font13/18 normal、border1；文字in160ms/y2→0，out100ms/easeOut，宽度spring visualDuration.24/bounce0，差<.5px忽略。 |
| 轮播与演示 | 控件1600/2800/3600/2000ms；官网演示1800/2800/3600/2200ms，Error恢复600ms。演示主图120px，stage240/小屏200；card max580；Avatar32+两枚Pill；尺寸12/16/20/24/32/48。手选停止轮播，Pause只暂停状态循环。 |

保留用户指定的8px矩形外壳、4px内嵌高亮及KUI语义色。原生提供像素/em/rem强类型适配，不伪称支持任意CSS字符串。GPUI 0.3.10没有公开独立aria-busy/live设置；采用动态名称及Status/Image角色，并把播报等价性列为待验。文字微光复用TextShimmer，字号13px，安装依赖已显式声明。原生7项组件测试覆盖快速中断、旋转归位、收拢/抖动、尺寸阈值、心跳/减少动效、Oklab透明混色、实际GPUI暂停及受控轮播；不将这些测试等同于逐帧视觉通过。

AiSpark 收尾记录：修复浏览器启动不足1秒时颜色初始化的时间减法溢出，初始相同颜色直接视为稳定值，不回填负时间。最终8项组件测试、2项story测试及Clippy通过；之前整体274项回归通过后增加了这项启动回归。空白项目通过CLI自动安装TextShimmer和其依赖，最终源码的原生与WASM离线编译均通过。复制源码仅有未使用共享辅助方法的dead_code警告。注册表233文件一致性及193项双语文档/795个静态目标通过；最终本地预览版本`9e7444681ca3545351e7`。中文实际画面确认首次启动可绘制、手选完成会停止轮播、射线朝前回正；仅网页截图保存在`/private/tmp/kui-ai-spark-reviewed.jpg`。未进行同尺寸60Hz逐帧和所有屏幕阅读器验收，仍保留为部分完成。

工作区格式边界：本轮改动的Rust文件通过rustfmt检查；整个快照的fmt检查仍报告其他组件既有格式差异，未改动它们。编译与生成使用隔离的稳定源码快照，其他Agent正在修改的文件不纳入本轮结论。

用户范围更新：此前108项按用户要求默认通过，不再复查。清单从Git提交`6c9a53ebfa16aa59c4ab388f18cbb944e8ea6594`的已分发catalog识别，排除foundation后恰为108项；当前总计193项，剩85项。持久进度见`component-audit-status.json`，桌面`KUI 组件验收进度.md`由`scripts/update-audit-status.py`更新。源码核对、原生/WASM、独立安装、视觉和动效分别记录。

## 用户追加：网页主题键月牙

用户指明是右上角调色盘旁的主题按钮，原 `M21 14.5 A8.5…` 路径的内弧半径7过于接近外弧半径8.5，造成月牙细扁。替换为官网公开渲染中使用的圆滑Moon轮廓，外弧半径9、内弧半径6；Lucide的ISC声明随网站静态资源提供。仅改网页导航SVG，不改变已经默认通过的组件范围。实测按钮前后均为34.59375×32 CSS px；本地环图页刷新后新路径生效，月亮与调色盘保持原位置。仅网页实拍：`/private/tmp/kui-header-moon-before-crop.jpg`、`/private/tmp/kui-header-moon-after-crop.jpg`。网站生成及193项双语文档/795静态目标检查通过，没有提交或发布。

本轮继续进行中的两项：Rating与RadioCards公开注册表TSX/CSS已取得，参数与键盘状态逐项核对。当前原生174项组件测试、104项story测试及5项其余测试/示例通过。评分文字模糊、宿主后续反馈完整中间态、选项卡片RTL及最终WASM/安装/视觉验收仍需继续；不标记为完整验收通过。共用Oklab颜色模型抽入`primitives/color_motion.rs`，AiSpark使用同一实现；安装注册表尚待最终生成与独立验证。
