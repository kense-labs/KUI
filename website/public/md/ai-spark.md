# AiSpark

以八道圆滑收尖光线表示助手的活动状态。原生与 Web 使用同一份 Rust + GPUI 控件。

## 接入

```rust
use gpui::prelude::*;
use kui::components::ai_spark::{AiSpark, SparkStatus, SparkVariant};

// 宿主控制实际任务状态；初次挂载不播放入场形变。
let mark = cx.new(|_| AiSpark::new("assistant")
    .with_status(SparkStatus::Thinking)
    .with_size(24.));
mark.update(cx, |mark, cx| mark.set_status(SparkStatus::Done, cx));

let label = cx.new(|_| AiSpark::new("activity-label")
    .with_variant(SparkVariant::Pill)
    .with_status(SparkStatus::Writing));
```

先调用 `kui::init(cx)`。复制源码使用 `kui add ai-spark`，会同时安装 `text-shimmer` 及其依赖；导入前缀改为 `crate::ui::`。

## 参数与默认值

| Rust API | 输入类型 | 默认值 / 行为 |
| --- | --- | --- |
| `new` | `impl Into<ElementId>` | 稳定 ID；没有显式状态时自动循环 Idle → Thinking → Writing → Done。 |
| `with_status` / `set_status` | `SparkStatus` / `(SparkStatus, &mut Context<Self>)` | 未设置；显式调用即进入受控状态，重复值也会停止自动循环，但不重新触发形变。 |
| `with_variant` | `SparkVariant` | `Mark`；另有 `Pill`、`Avatar`。 |
| `with_size` | `f32` | Mark 24px、Pill 16px、Avatar 32px。正有限数，不限制为12–128px。Avatar 图形为 `round(size×.56)`。 |
| `with_em_size` / `with_rem_size` | `f32` | 未设置；分别乘继承字号 / 根字号。相对尺寸的 Avatar 图形为 size×.56，光学提示使用实际图形像素的四舍五入值。 |
| `with_tone` | `SparkTone` | `Accent`；另有 `Foreground`、`Muted`、`Inherit`。读取 KUI 语义色。 |
| `with_color` | `Hsla` | 未设置；覆盖 tone。Error 始终采用 danger72% + text_muted28% 的 Oklab 混色。 |
| `with_speed` / `set_speed` | `f64` / `(f64, &mut Context<Self>)` | 1；仅接受有限数；控制循环时间和旋转目标速度，不改变状态混合、成功收拢或失败抖动的时钟。 |
| `with_twinkle` / `set_twinkle` | `bool` / `(bool, &mut Context<Self>)` | true；控制 Idle 的缓慢闪烁。 |
| `with_paused` / `set_paused` | `bool` / `(bool, &mut Context<Self>)` | false；暂停图形与内置状态轮播，恢复时从当前姿态继续。Pill 的独立文字微光不受此值控制。 |
| `with_announce` | `bool` | true：Status 角色；false：具名 Image 角色。 |
| `with_aria_label` | `impl Into<SharedString>` | 未设置；覆盖可访问名称。 |
| `with_label` | `impl Into<SharedString>` | 未设置；覆盖 Pill 文字，Mark / Avatar 不显示此文字。 |
| `with_labels` | `[impl Into<SharedString>; 5]` | `Ready / Thinking / Writing / Done / Couldn't finish`。 |
| `with_aria_labels` | `[impl Into<SharedString>; 5]` | `AI ready / AI is thinking / AI is writing / AI finished / AI couldn't finish`；KUI 本地化扩展。 |
| `decorative` | `bool` | false；true 移除根状态角色及名称，KUI 扩展。 |
| `set_auto_play` | `(bool, &mut Context<Self>)` | true；恢复时使用内置轮播的当前索引。 |
| `set_visible` | `(bool, &mut Context<Self>)` | true；宿主隐藏时冻结图形和文字微光；内置状态计时独立。 |
| `status` / `is_auto_playing` | `()` | 读取有效状态 / 是否启用内置轮播。 |

状态名 `Writing` 对应参考实现的 `generating`，保留既有 KUI Rust 调用方式。Rust 输入使用强类型，不接收任意 CSS 长度或 CSS style / className；布局样式由 GPUI 宿主负责。尺寸非法时保留已有值。没有点击回调，完成状态由实际任务驱动。

## 布局

- 100单位网格，8道射线每道间隔45°；轮廓是两个圆的凸包，端头与根部圆滑。
- 光学权重 `clamp(log(max(1,size)/12)/log(8),0,1)`。低于18px时，对角射线改为圆点；28px起显示中心点；40px起允许生成态粒子。
- Pill 高28px、字号13px、普通字重、行高18px、间距8px、左右内边距7/12px、边框1px。文字按实际字形测宽，不写死英文宽度。
- Avatar 背景为图形色8% + surface92%；Error 改为 danger7% + surface93%，均用 Oklab 混色。
- 矩形外壳沿用 KUI `radius.md` 8px。图形中的圆点保持圆形；这是用户指定的主题差异。

## 动效

- Idle 闪烁周期6.4s、中心呼吸3.2s；Thinking 波动1.9s、整体呼吸4s；Writing 心跳1.04s，第二拍强度.62，分组相位延迟.045、对角延迟.17。
- Thinking / Writing 目标旋转速度26 / 46°/s，以 `1-exp(-3dt)` 追近。离开忙碌态后按当前速度朝前滑行到90°倍数，时长限制.6–1.8s，不突然跳回0°。
- 切入 Writing / Done / 其他状态分别混合420 / 500 / 560ms，使用 cubic-in-out。快速反切保留当前混合权重、循环相位、角度和角速度。
- Done 收拢680ms：170ms收拢、510ms释放；主/对角长度减幅.12/.20，内移2.4/3单位，整体缩小.045，中心放大.35。Error 横向抖动460ms，振幅3.2单位、周期150ms、平方衰减。
- 文字入场160ms、y=2→0、standard(.22,1,.36,1)；离场100ms easeOut；宽度240ms临界弹簧；忙碌态复用 TextShimmer。首次挂载文字直接显示。
- 图形色、Avatar 背景过渡320ms standard；减少动效时160ms。减少动效停止所有循环、粒子和冲击，角度取最近90°，状态姿态仍混合200ms，文字淡入100ms且不位移。CSS160ms与姿态200ms是参考实现中两个独立参数。
- 帧 delta 最大1/30s；离屏检测扩展48px；暂停、宿主隐藏及窗口隐藏均不累计图形时间。静止结果不持续请求下一帧。
- 安装控件的内置轮播间隔1600 / 2800 / 3600 / 2000ms；展示页间隔1800 / 2800 / 3600 / 2200ms。展示页“暂停”只暂停状态轮播，射线继续运动；手动选状态会停止轮播。

## 验收与边界

覆盖中途换状态连续性、旋转收尾、重复结果不重播、尺寸阈值、两次心跳、减少动效、Oklab透明色和实际 GPUI 暂停/轮播用例。GPUI 0.3.10 公开构建器没有独立 aria-busy / aria-live 设置，当前使用 Status / Image 与动态名称；尚未证明屏幕阅读器与 DOM 版本播报完全等价。未完成同尺寸60Hz逐帧误差验收。
