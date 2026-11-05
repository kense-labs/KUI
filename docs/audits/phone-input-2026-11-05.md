# 电话字段国家列表修复

2026-11-05，用户反馈国家列表无法上下滚动，缺少国旗，并明确要求阅读官方源码。

## 依据

已读取[公开注册表源码](https://uiarc.dev/r/phone-input.json)的TSX与CSS。SHA-256：`9e4d868814c05693847f3c908a84a13e8fe64e174456c5feb239b489dcaf2240`。

核对参数：列表max-height280px；选项min-height36px、gap2px、横向padding10px、内容gap10px；国旗槽宽20px（参考站使用17px Emoji）；国家名medium并省略长文字，区号12px；勾选标记16px。高亮弹簧0.28s，打开/收起0.4s/0.32s；打开时滚入当前国家、键盘移动时滚入目标，方向键在边缘停止，PageUp/PageDown跨8行，空查询时Home/End定位首尾。矩形圆角按KUI规范保留8px。

## 根因与修改

原列表使用overflow_y_scroll但没有ScrollHandle、明确的非收缩行或网页滚动区域注册。Web预览捕获阶段将未注册区域的滚轮交还正文，因此列表无法接收滚轮。现绑定滚动位置、向已有EditorScrollRegion报告实际边界及方向，并保持搜索框固定。国家选项只绘制共享高亮，去掉表单按钮自带的另一层悬停底色。

国旗采用MIT许可flag-icons固定版本的4:3 SVG，内嵌到Rust primitive，20×15px绘制、20px占位；SVG固有尺寸80×60，防止小图标被解码成大纹理。保留完整许可，安装清单显式带入phone_flags。原生/Web不依赖Emoji字体或运行时网络。参考站用Emoji，这里使用原生可稳定绘制的彩色SVG；不宣称两种图像渲染逐像素相同。

新回归覆盖：真实滚轮、触摸拖动/取消、Home/End可见性、关闭后不拦滚轮、筛选提交/重开定位、空结果不提交、49个国家图像完整及句柄缓存。测试还发现清空旧搜索的Changed事件会覆盖重开选中行，现已处理。

## 验证

3项新增原生测试通过。WASM、独立安装、注册表与实际浏览器结果在完成后继续补充。本次仅更新本地，不提交推送或发布。

实物检查追加：选择中国后，原输入框仍显示美国号码占位样例。现从同一公开TSX的49项country数据与mask规则提取各国示例，初始化及切换国家时同步占位示例；不扩展或声称已实现完整号码格式化/验证。

## 完成记录

- 最终本地预览版本：`36a6333878f2c9ba27b8`。真实浏览器已确认中国国旗、`+86`与`131 2345 6789`同时更新；首页列表49个国家的国旗完整。
- 浏览器实测列表内滚轮向下、反向滚回，正文scrollTop保持0；Home/End能定位首尾且越界不循环，`+86`筛选与Enter提交正常。亮暗主题、320px窄宽检查通过，无横向溢出。浏览器触屏未实际模拟；原生测试已覆盖触摸拖动与取消。
- 3项针对性原生测试通过；库与Web的WASM检查、release WASM打包、注册表生成与校验、网站构建与站点检查通过。站点检查覆盖193项中英文组件、795个静态目标；235个预览文件哈希与源码一致。
- 新空白项目`/private/tmp/kui-phone-install-afu_p2w7`执行`kui init`、`kui add phone-input`、doctor、原生与WASM编译均通过，自动带入国旗primitive。
- 截图：`/private/tmp/kui-phone-final-list.jpg`、`/private/tmp/kui-phone-final-china.jpg`；滚动实拍：`/private/tmp/kui-phone-list-scrolled.jpg`。
- 范围外现状：全仓Clippy仍有其他组件的6项既有错误，本次未修改对应文件；不声称全仓所有检查通过，也不代表恒定60fps或Emoji逐像素一致。

仅更新本地站点，未提交、推送或发布。
