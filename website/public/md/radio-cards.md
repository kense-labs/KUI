# Radio cards

带说明、图标和尾部信息的单选卡片。安装标识为 `radio-cards`；桌面与 Web 共用 Rust + GPUI 状态、布局和绘制。

## 安装与使用

先调用 `kui::init(cx)`。库使用 `cargo add kense-kui --rename kui --no-default-features`；复制源码使用 `kui init`、`kui add radio-cards`，将导入改为 `crate::ui::`。

```rust
use gpui::prelude::*;
use kui::components::radio_cards::{RadioCardOption, RadioCards};

let items = vec![
    RadioCardOption::new("one", "One bag")
        .with_detail("12 oz, whole bean").with_price("$18"),
    RadioCardOption::new("two", "Two bags")
        .with_detail("Two origins each month").with_price("$34"),
];
let control = cx.new(|cx| {
    RadioCards::new("Plan", items, cx)
        .with_min_column_width(200.)
        .with_selected("two")
        .with_name("plan")
        .with_required(true)
});
```

## Rust API 与默认值

| 接口 | 类型 / 默认 | 约定 |
| --- | --- | --- |
| `new(label, items, cx)` | `impl Into<SharedString>, Vec<RadioCardOption>, &mut Context<Self>` | label 为组的无障碍名称；可见组标题由宿主绘制。 |
| `with_layout` | `RadioCardLayout = Grid` | `Grid` 为响应式网格，`List` 为单列。 |
| `with_min_column_width` | `f32 = 180.` | 接受正有限像素值；忽略无效输入。窄于最小列宽时仍以可用宽度显示一列。 |
| `with_selected` | `&str`，默认无选择 | 初始选中的稳定 key；不存在的 key 不生效。允许宿主显示已选中但已禁用的选项。 |
| `with_disabled` | `bool = false` | 禁用全组，不清空宿主选择。 |
| `with_required` | `bool = false` | 启用 `is_valid` 的非空校验；不自动选择第一项。 |
| `with_controlled` | `bool = false` | `true` 时仅发送选择意图，等待 `set_selected` 确认；未确认前保持原选中轮廓。 |
| `with_name` | `impl Into<SharedString>`，无 | 宿主序列化的字段名。 |
| `selected()` | `Option<SharedString>` | 当前已提交 key。 |
| `set_selected(key, cx)` | `(&str, &mut Context<Self>) -> bool` | 宿主赋值，不发事件；未知 key 返回 false。 |
| `clear_selected(cx)` | `&mut Context<Self>` | 清空选择和移动轮廓的已定位状态。 |
| `is_valid()` | `bool` | 非必填或已有选择为 true；提交和错误提示由宿主完成。 |
| `form_value()` | `Option<(SharedString, SharedString)>` | 无名称返回 None；有名称但未选择时值为空字符串。 |
| `RadioCardChanged(pub SharedString)` | GPUI 事件 | 点击或键盘选择不同的可用 key 时发送。重复选择不发送。 |

`RadioCardOption::new(key, label)`：key、label 为 `SharedString`，`detail`、`price` 默认空，`icon: Option<Glyph>` 默认 None，`disabled` 默认 false，`disabled_reason: Option<SharedString>` 默认 None。

| 选项 builder | 参数 | 行为 |
| --- | --- | --- |
| `with_detail` | `impl Into<SharedString>` | 主标签下方说明。 |
| `with_price` | `impl Into<SharedString>` | 任意尾部信息，不做价格解析；网格中在底部，列表中在尾部。 |
| `with_icon` | `Glyph` | 主标签前 18px 图标。 |
| `with_disabled` | `bool` | 该项不可选择。 |
| `with_disabled_reason` | `impl Into<SharedString>` | 设置原因并禁用此项，保留现有 builder 语义；禁用时替换普通说明。 |

## 尺寸与布局

- 外卡、移动轮廓均读取 KUI `radius.md = 8px`，radio 圆点保持圆形。外卡边框 1px；轮廓 1.5px，基于实际 GPUI 排版边界绘制，不累计猜测宽度。
- 网格 gap 10px，列数遵循 `auto-fill` 语义：`floor((可用宽度+10)/(最小列宽+10))`，至少一列，各列等宽；奇数末项保留单列宽度。卡片 padding 14px 16px，主体与圆点横距 12px；底部信息前距 10px，字号 18px，行高 1.4。同行卡片的底部信息对齐。
- 列表 gap 8px；padding 为上/下 13px、左 14px、右 16px；圆点 / 主体 / 信息横距 12px，垂直居中。
- 标签 14px、medium、1.4 行高，长标签省略；标签与说明相距 2px；说明 12px、1.4 行高。圆点外框 18px、描边 1.5px，内点 6px，网格顶距 1px。图标 18px，与标签间距 8px。
- 选中背景为 `control-on` 4% 与 surface 的 Oklab 混色。禁用背景用 surface-muted，标签和尾部信息用 text-muted；禁用的圆点边框用 border。

## 交互、动效与验收

只有一个可用项进入 Tab 顺序。方向键循环选择并跳过禁用项；Home / End 到首个 / 最后一个可用项。Enter / Space 选择；全部禁用时无 Tab 入口。点击选中项不重复发送事件。受控模式中焦点可到新项，已提交值仍等待宿主确认。书写方向为 RTL 时 Left / Right 反转，Up / Down 与 Home / End 不变；网格逐行镜像，导航仍按块顺序。

指针仅在可悬停设备上产生悬停边框；未选中项按下或键盘可见焦点时使用 surface-muted。背景、卡边框、圆点颜色为 0.16s、`ease(0.22,1,0.36,1)`。圆点缩放为 0.26s、bounce=0；共享外轮廓位置和尺寸为 0.42s、bounce=0。首次定位仅 0.16s 淡入，不从原点滑来；容器尺寸变化时直接贴合新布局。反向选择保留当前弹簧速度；减少动效时直接应用全部状态。首次排版测量完成前网格保持布局但零不透明度，避免首帧按错误列数绘制。

重点验收：网格最后单项不拉伸、320px 单列、变宽后轮廓不漂移、禁用项跳过、受控拒绝后值不变、快速跨行切换，以及 RTL 下的镜像与按键反转。完整状态见[进度记录](../audits/component-audit-status.json)。

2026-11-05 网页动效实测（900×760 浅色英文，`?story=radio-cards`，以 0.1 倍速时钟抓帧）：

- 默认选中「Two bags」（卡片 x347..552）。点击「One bag」后，共享选中环从 348..551 平移到 131..335（位移 217px，即 206.67px 卡片宽加 10px 网格 gap）。左右两条边逐帧同幅移动，属刚性平移而非拉伸。
- 途中约 20 个中间帧，帧间位移依次 5.9 / 6.7 / 8.0 / 6.7 / 6.0 / 4.4 / 4.2 / 4.2 / 3.7 / 3.1 / 2.9 / 3.0 / 2.6 / 2.3 / 2.4 / 2.1 / 1.9 / 1.7 / 1.3 / 1.0 / 1.0 / 1.0 / 0，单调减速、无回弹，与 `radio_cards.rs` 的 Slide 重定向（`duration(x, 0.42)`，ω = 5.236 / 0.42 = 12.47 临界阻尼）一致；点击到完全静止约 0.61s，为名义 0.42s 可视时长的约 1.44 倍，符合该弹簧模型。
- 圆点填充来自 `indicators[i].duration(selected, 0.26)`：实测旧点墨量 244→224→60→0、新点 0→76→267→286（过冲）→244（回落），并有两帧同时可见旧点半清、新点半满的交叉淡入淡出，非硬切换。
- 减少动效：同一路径下环在单帧内由 348..551 直接变为 131..335，无中间帧，印证 `disabled_motion` 时 `reset` 直贴目标。
- 本次只覆盖该路径的连续动效；深色、320px、125% 缩放与触摸下的动效未复测，也未与同速 60Hz 基线逐帧比对，故不声明帧级一致。

当前边界：GPUI 固定列网格需要首轮排版测量后更新列数。首帧不跳位已在浏览器实测：900px 视口下连续抓取 40 帧、间隔 55ms，第 0 帧只含 List 分组，网格区域非背景像素恰为 0，第 1 帧起逐字节一致（稳定为两列 206.67px + 一行一项），说明已测量的列数就是用户看到的第一帧；外层网格与选中轮廓同为该帧扣留，轮廓不再按测量前的错误列数绘制。

2026-11-05 补充网页实测（900px 英文浅色，选中项以实心圆点定位，逐帧比对确认已静止后再判定）：

- LTR（视觉序 One bag / Two bags / Bulk = 逻辑 0 / 1 / 2）：点选视觉左卡得 L，按 → 得 M、← 得 L，从视觉中卡起 → 得 R、← 得 M、再 ← 得 L，Home 得 L、End 得 R。方向键与 Home/End 均落在逻辑序上。
- RTL（列镜像为 Bulk / Two bags / One bag）：点选视觉左卡得视觉 L、按 → 得视觉 M、← 回视觉 L；从视觉中卡起 → 得视觉 R、← 得视觉 M、再 ← 得视觉 L；Home 得视觉 R、End 得视觉 L。
- 结论：镜像与按键反转在设计上互相补偿，横向键在两种方向下都保持「→ 向视觉右、← 向视觉左」的一致手感，而 Home/End 始终指向逻辑首/末项（LTR 下为视觉左、RTL 下为视觉右）。因此纯逻辑序的反转无法从像素位置单独观测，它由 `radio_cards.rs` 的 `horizontal_arrows_reverse_only_under_rtl` 单元测试（断言 Rtl 下 → 取逻辑首项）与 `key()` 中的 `dir.inline_step(±1)` 保证，该测试通过；浏览器实测只证明视觉方向一致与 Home/End 的逻辑端点语义。
- 125% 缩放（`deviceScaleFactor=1.25`）：卡片边框与圆点正常、无错位，布局仍为三列。
- 触摸（`Input.dispatchTouchEvent`）：轻点视觉左卡后选中态改变（帧内容变化），命中正确。

首帧稳定性目前覆盖 900px 英文浅色与 320px 深色中文；125% 缩放与触摸下的首帧与方向未逐项实测。上述动效采样未与同速 60Hz 基线逐帧比对，故不声明帧级一致。富文本标签由宿主转换为原生数据，目前公开选项使用 SharedString。宿主负责表单提交；GPUI 无障碍桥接不能完整表达 HTML 的 required/disabled 属性。不得将这些边界写成视觉和无障碍已完全通过。
