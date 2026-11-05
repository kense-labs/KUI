# Rating

评分控件：支持整星、半星、鼠标拖选、键盘和只读展示。安装标识为 `rating`，桌面与 Web 使用同一份 Rust + GPUI 实现。

## 安装与使用

初始化 `kui::init(cx)`。库依赖使用 `cargo add kense-kui --rename kui --no-default-features`；复制源码使用 `kui init`、`kui add rating`，并将示例导入改为 `crate::ui::`。

```rust
use gpui::prelude::*;
use kui::{components::rating::Rating, primitives::Size};

let control = cx.new(|cx| {
    Rating::new("Quality", 5, 3.5, cx)
        .with_half(true)
        .with_size(Size::Md)
        .with_name("quality")
});
```

## Rust API 与默认值

| 接口 | 类型 / 默认 | 约定 |
| --- | --- | --- |
| `new(label, max, value, cx)` | `impl Into<SharedString>, usize, f32, &mut Context<Self>` | 名称、最大星数、初始值均为显式参数；常用 `5, 0.`。不限制为最多十星。 |
| `with_half` | `bool = false` | 步长由 `1` 改为 `0.5`；初始值和宿主值按最终步长四舍五入、限制在 `0..=max`。 |
| `with_allow_clear` | `bool = true` | 再次点击当前分值可清除；Backspace / Delete 清除。键盘 Enter / Space 选择而不反选。 |
| `with_show_value` | `bool = true` | 显示预留宽度的读数，改变文字时不推动相邻内容。 |
| `with_size` | `Size = Size::Md` | 见尺寸表。 |
| `with_description` | `impl Into<SharedString>`，空 | 标签下方说明，亦用于交互组的无障碍描述。 |
| `with_empty_label` | `impl Into<SharedString>` | 未设置时按语言显示 `Not rated` / `未评分`。 |
| `with_value_labels` | `Vec<SharedString>`，空 | 从 1 星起按下标取整星文案。默认五颗整星为 Poor / Fair / Good / Great / Excellent；半星模式默认使用数字。 |
| `with_read_only` | `bool = false` | 仅图像语义，无焦点和输入；无障碍名称包含标签和数值，例如 `Quality: 4 out of 5`。 |
| `with_disabled` | `bool = false` | 阻止用户输入；星标和读数透明度 0.5，标签和说明不变。 |
| `with_controlled` | `bool = false` | `true` 时发送选择意图，等待宿主 `set_value` 后才更新已提交数值。 |
| `with_name` | `impl Into<SharedString>`，无 | 设置宿主表单字段名。 |
| `value()` | `f32` | 返回归一化后的当前已提交数值。 |
| `set_value(value, cx)` | `f32, &mut Context<Self>` | 宿主赋值，不发送 `RatingChanged`；忽略非有限值。 |
| `form_value()` | `Option<(SharedString, f32)>` | 未设置名称返回 `None`；由宿主序列化，不隐式提交 HTML 表单。 |
| `RatingChanged(pub f32)` | GPUI 事件 | 只在选择意图不同于当前值时发送；受控宿主负责确认或拒绝。 |

构造器的非有限初始值归零。`max = 0` 呈现空星组；应用应为可编辑评分提供正的 `max`。

## 尺寸与绘制

| Size | 每星命中区域 | 星形尺寸 | 组间隙 |
| --- | --- | --- | --- |
| `Sm` | 28 × 28 px | 18 × 18 px | 2 px |
| `Md` | 36 × 36 px | 24 × 24 px | 2 px |
| `Lg` | 44 × 44 px | 30 × 30 px | 2 px |

星组左右用 `(星形尺寸 - 命中尺寸) / 2` 的负边距，将可见星边缘对齐标签；不能缩小实际命中区域。星形使用 24 单位矢量坐标和 1.75 单位圆头描边，分别绘制底层、已提交淡色层和预览实色层；半星裁剪填充与描边。标签 14px、1.4 行高、medium，字段纵向间隙 6px；说明 12px、1.4 行高。文字与星组间距 8px，换行纵距 2px。

## 输入、状态与动效

- 悬停仅预览，离开星组恢复已提交值。鼠标按下后跨星拖动，在松开时提交一次；跨过其他值又回到起点仍视为拖选，不误触清除。
- 半星拥有各自的 radio 焦点。只有当前选择（空值时第一步）是 Tab 入口。左右 / 上下调整步长，Home 选第一步，End 选最大值；数字 1–9 在范围内直接选择。允许清除时 Backspace / Delete 归零。键盘 Enter / Space 只选择、不反选：GPUI 0.3.10 会在焦点元素的 key-up 上合成一次点击，组件因此只让指针点击走清除路径，键盘激活一律按普通选择处理，否则按住 Enter / Space 会一边读取一边把已提交分值清零。
- 触摸先辨认方向；纵向意图交还宿主滚动，横向拖选才认领。取消和失焦不提交，触摸合成点击不重复提交。
- 填充为 `visualDuration = 0.26s, bounce = 0` 弹簧，逐星错开 0.035s；已提交层为 0.4s 弹簧。按压缩至 0.86，未亮起星缩至 0.9。独立提交回弹从 scale=1、速度=9/s 开始，间隔不超过 260ms 时只弹所选星。
- 文案按词保留相同部分；变词入场 0.24s、`ease(0.16,1,0.3,1)`、位移 0.3em，退出 0.12s、`ease(0.22,1,0.36,1)`、反向 0.25em。读数预留所有可能文案的最宽字形宽度。
- 减少动效：填充和缩放直接到位，禁止提交弹跳，文字不滚动。首次显示已有分值直接初始化，不播放从零填充。

演示页的反馈表单属于 story：改分后等待 240ms，1–3 星展开说明，4–5 星显示感谢；快速改分取消前一计时，清零重置草稿。暂不填写保留草稿，再次编辑继续使用；空白说明不能发送。

## 验收与当前边界

需验证半星独立焦点、跨星拖选、受控宿主确认、尺寸和触摸取消，以及说明的取消计时、草稿保留、发送后焦点。完整视觉和连续动效验收以[进度记录](../audits/component-audit-status.json)为准，原生测试通过不等于已完成视觉验收。

组件读数用多次偏移绘制近似 CSS blur：变词入场 4px→0、退出 0→2px，旧词同时反向位移并交叉淡出，均已按词实现；该近似已在网页实测到逐帧衰减（见下），但它是对浏览器原生 blur 的近似，二者像素级一致性仍未验证。story 说明面板已具备退出内容的交叉淡出：旧内容用独立的 0.12s `ease(0.22,1,0.36,1)` 曲线淡出，新内容延后 0.06s 起用 0.24s 入场，两者位移同向（新内容自 -8px 下落至 0，旧内容继续下行 0→6px）；浏览器实测在过渡中帧可同时看到旧「Thanks, Emma will see your rating / Add a note」呈半透明残影、容器高度继续增长，非硬切换。面板本身仍无入场的 2px 模糊（GPUI 0.3.10 无内容模糊，只能用不透明度与位移近似）。GPUI 0.3.10 的无障碍桥接尚不能完整表达 Web `aria-live`、独立 disabled 属性及原生隐藏表单输入。不得把这些差异标为已通过。

2026-11-05 网页实测（900×760 浅色英文，`?story=rating`）：

- 悬停第 3 星预览满 3 星并显示 `Good`，离开恢复已提交值；按下第 4 星时该星满宽但高度只剩 21→按下缩放中，回弹后恢复；跨星拖选到第 5 星提交 5，拖出后回到起点再松开仍为 1，未误触清除。
- 键盘：点击第 1 星获得焦点后 → 逐步加到 `Good`（3/5）；Home 归 1、End 到 5、数字 4 直达 4、Backspace / Delete 清零；已提交 4 后按 Enter 或 Space 连按两次仍保持 4，方向键仍可加；指针再次点击同一星仍按 `allow_clear` 清零。这一项此前是缺陷（Enter / Space 会清空分值），已在 `registry/src/components/rating.rs` 修复并重建 WASM 复测。
- 触摸：`Input.dispatchTouchEvent` 轻点第 3 星提交 3/5 并展开说明面板。
- 深色：悬停与已提交态的橙色描边正常，禁用态全灰；深色满星像素数 232，浅色 261（描边混色差异，非几何差异）。
- 减少动效：已提交 4 与悬停 3 均与常规态一致，无过渡帧差异。
- 320px 窄屏中文：五颗星横排不换行，「非常棒」与「谢谢，Emma 会看到你的评分 / 补充说明」均完整，未出现方框缺字。
- 125% 缩放：星形 22.4 CSS px（100% 下 22）、星距 37.6（100% 下 38），均在 1 CSS px 内。
- 逐星错开的填充与变词已采样：以 0.1 倍速时钟（`performance.now` 缩放）抓取 18 帧，每颗星依次晚约 1 帧到达峰值并各自过冲到约 291 像素后回落到 261，形成逐星错开；第 4 帧同时可见第 3 星半填充、第 4 星淡入、旧词「Good」残影与新词叠加、说明面板仍在增高。该采样只覆盖变词与逐星填充的中间态，未与同速 60Hz 基线逐帧比对，故不声明帧级一致。

半星独立焦点、受控宿主确认与尺寸表仍只在单元测试覆盖，尚未逐项网页实测。

2026-11-05 补充网页实测（沿用同一 0.1 倍速时钟）：

- 说明面板退出交叉淡出已抓到同时含新旧内容的中间帧：点击「Not now」后连拍 40 帧，`f00` 仍为完整说明表单（含「What would have made it smoother?」标题、textarea 与「Not now / Send note」按钮行），`f01` 一帧内同时可见旧说明内容（标题、placeholder、按钮均呈半透明并整体下移）与新「Thanks, Emma will see your rating / Add a note」行淡入，`f03` 起旧内容完全消失、只剩新行，证明是交叉淡出而非硬切换，与 `selection_demos.rs` 的 `exit = standard_ease(age/0.12)`、`exit_y = exit*6`、旧 body 以 `1.-exit` 淡出、新 body 延后 0.06s 入场一致。
- 面板高度退出实测：面板底部由 437 逐帧收敛到 286，帧间位移 42 / 42 / 31 / 17 / 10 / 4 / 3 / 1 / 1 / 0，单调减速、无回弹、无跳变，与 `Slide::duration(target, 0.4)` 临界阻尼模型一致。
- 125% 缩放（`deviceScaleFactor=1.25`，即画布读取的 DPR 路径）：五颗星字形宽 27 CSS px、星距 47（100% 下 21 / 38），均在 1 CSS px 容差内，星级横排不换行、读数完整。
- 读数变词模糊实测（560×300 视口、`deviceScaleFactor=3` 放大亚像素、0.05 倍速时钟，裁剪读数带 96×30 CSS px 后逐帧只录屏、事后再解码）：点第 3 星使读数由 `Not rated` 变 `Good`。以「软边像素数」（字形笔画边缘的中间灰像素，直接反映多 tap 偏移的扩散宽度）为指标，逐帧 3462 → 2167 → 1427 → 561 → 520 单调收敛，最暗像素同步由 155 收紧到 82，第 6 帧起帧字节完全一致（`f83ea6c8`）；即 `enter=(1.)` 后 `blur=4*(1-enter)→0` 的入场模糊近似确实逐帧衰减并终止于单一清晰字形，与 `rating.rs` 中 `taps = if blur>0.22 {5} else {1}`、偏移 `±0.7*blur` 的实现一致。同组帧中读数最左墨迹起点稳定在 8 设备 px（首帧 10/8 的抖动为旧词退出残影），印证读数预留最宽字形宽度、变词不推动相邻内容。
- 触摸（`Input.dispatchTouchEvent`，移动端触点模拟）：轻点第 3 星后提交成功并展开说明面板，帧内容改变（md5 变化），未出现误触清除。
- 说明面板按钮行为（暂不填写保留草稿、空白不可发送、发送后焦点落在「编辑说明」）由 `selection_demos.rs` 的 `note_skip_send_edit_preserves_the_draft_and_focus` 覆盖并通过；快速改分取消前一计时由 `rating_follow_up_cancels_stale_delays_and_clearing_resets_the_draft` 覆盖并通过。

半星独立焦点、受控宿主确认、尺寸表以及无障碍桥接仍只在单元测试或源码层面覆盖，尚未逐项网页实测，故不标为视觉/动效完全通过。
