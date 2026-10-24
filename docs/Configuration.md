# 配置与 Forge 菜单

现行构建只有 Presentation 和 Moveset 两个 NRO。4G菜单及新动作已合并到Moveset；旧Style NRO需要停用，旧源码保留作历史参考。

INI唯一位置是MHGU当前用户的存档根目录：`cb_presentation_to_4g.ini`、`cb_moveset_to_4g.ini`。默认模板在项目`conf/`中，首次运行生成。Forge菜单和手动编辑读写同一份文件；功能设置保存后重启应用。原版风格菜单的4G选择即时应用并保存。

## 公共资源始终加载

不论`[plugin] enabled`、Hitdata或SAED是否启用，都注册盾斧动作`0xAA/0xAB`，并将盾斧HDP从80条扩为81条、追加`0x50`。已有81条时更新末条，不重复扩容。其它武器HDP不处理。

Hitdata关闭时保留加载文件的前80条；开启且插件总开关开启时才复制内嵌数据，并把`0x4F`恢复原版106/6/55。新增`0x50`固定为延迟0、持续8、动作值100。

## 按风格编号配置

六个功能只用`[styles]`中的编号集合控制，各自没有第二个功能总开关。只保留0..7八个编号。黄盾变正常AED默认选武士道3和4G6，GP后退/防性默认只选6，新增后三项默认none；`[plugin] enabled`仍是整个插件战斗功能的总开关。

| 编号 | 风格 |
|---|---|
| 0 | 公会 |
| 1 | 强袭 |
| 2 | 空战 |
| 3 | 武士道 |
| 4 | 炼金 |
| 5 | 勇气 |
| 6 | 本项目4G；底层使用公会资源 |
| 7 | 预留风格 |

```ini
[styles]
yellow_shield_aed = 3,6
gp_recoil = 6
guard_performance = 6
shield_thrust_yellow = none
guard_recoil_yellow = none
aed_followups = none
```

支持`all`、`none`、`0,2,4,6`、`0-7`、`all,!3`。all只包含这八个编号；8及以上会报配置错误，不增加菜单选项。例如`shield_thrust_yellow = 6,7`只选择4G和预留编号，none关闭该功能。GP后退或防性列表为none时，不安装相应桥接。

Forge菜单上方的“Moveset”提供0–7的Style ID选择器和六项复选框，勾选直接添加当前编号，取消直接移除；J guard是独立全局开关，默认开启。4G选择只在本机公会盾斧上映射为6，远端公会仍按0处理。

菜单及保存后的INI均按“Moveset派生/风格 → SAED → 数值”排列。数值区包含盾补正、固定震动和Hitdata。已有明确填写的列表保留；要应用新的默认值，在Forge菜单点击Defaults、Save INI，再重启游戏。

`0x44/0x4B`是原黄盾前置动作。盾突、J guard或防御后路由仅改变它们的入口，不能独自变成普通AED。还需为同一风格勾选“Yellow shield becomes normal AED”，才把0x44/0x4B转成新增0xAA/0xAB；之后由原盾状态/有瓶判定进入普通大解或超解。

## Moveset功能

| 键 | 默认值 | 行为 |
|---|---|---|
| `[plugin] enabled` | true | 战斗功能总开关；公共动作、HDP和风格菜单始终注册 |
| `[styles] yellow_shield_aed` | 3,6 | 当前风格在列表中时，将临时命令0x44/0x4B改为0xAA/0xAB，保留输入条件与窗口；不再写旧七处黄盾补丁 |
| `[styles] gp_recoil` | 6 | 当前风格在列表中时不ORR 0x20，GP成功退后；其它风格执行原ORR。动作0x54狩技始终保留原置位 |
| `[styles] guard_performance` | 6 | 当前风格在列表中时，白/红盾有效GP独立减威力10（最低1），红盾使用40/50阈值；否则完整运行原GU回调。黄/蓝盾保留原规则 |
| `[styles] shield_thrust_yellow` | none | 所选风格盾突链0x7FA第1条X+A接黄盾0x44，保留窗口；强袭原链为空，不新增盾突动作 |
| `[styles] guard_recoil_yellow` | none | 所选风格小退0x802/中退0x803后追加X+A接黄盾0x4B；原表全部保留，大退不添加 |
| `[styles] aed_followups` | none | 仅所选风格保留原大解及红盾取消命令，再追加本风格0x7DC第1条起的派生，排除第0条弹刀 |
| `[moveset] j_guard_aed` | true | J guard X+A接黄盾；仅武士道J防成功链0x81A第3条接0x44，Pre20/Start30/End0保持 |
| `[hitdata] enabled` | true | 覆盖前80条的独立开关，不控制新增0x50 |
| `[style] selected` | false | 原版菜单选择4G时自动保存；不作为额外战斗补丁开关 |
| `[values] shield_multiplier` | true | 红/蓝盾补正档位统一2，即1.2；黄盾档位0，即1.0。对新增风格也生效 |
| `[values] aed_camera_shake` | true | 大解瓶爆震动从15改为固定20，仅提供启停 |

GP判断仍核对`+0x333C=1`、`+0x2510&0x10`和实际GP动画，不依赖后退会取消的`+0x3340&0x20`。普通防御及遗留阶段不会得到额外GP减值。

旧INI的三项`[moveset]`布尔总开关只作迁移读取，不再形成运行时第二层判断。已有`[styles]`列表时，以列表为准，与节的顺序无关；缺少列表时，旧false迁为none，旧true使用对应项新默认。保存时不再输出这三个布尔键。

旧`[moveset] bushi_shield_thrust_aed`、`[aed_followups] enabled`、`[overpowered] enabled`只迁移验证格式，不再作为全局开关；缺少新列表时用none，不自动全开八个风格。需要用新列表明确选择；保存时不再输出这些旧键。

旧`[moveset] shield_multiplier/aed_camera_shake`仍可读取，保存移到`[values]`。旧`aed_shake_strength`允许读取但数值不再生效，保存时删除；开启震动固定20。

## 小退与中退的追加方式

原公共动作`sub_602C5308`（小退，动画5211）和`sub_602C5488`（中退）调用虚表`+0x63C`，参数分别为0、1。盾斧该槽为main+0x011D4E48，初始化链0x802、0x803。

在InitMoveChain成功后为当前玩家建立表副本，先保留所有原记录，再借用本风格第1条的Active与帧窗口追加末条。小退Start30，中退Start42；Pre继承当前资源。读到该末条时，main+0x002EA87C桥接只改临时命令前24字节为`0x10/0x4B/param0/type8/input0x1B`，保留Pre/Start/End。不改ARC、共享命令或其它原派生，重复初始化不重复追加。

## 菜单文案

| 英文 | 含义 |
|---|---|
| Yellow shield becomes normal AED | 黄盾变正常大解 |
| Use GP recoil in this style | 当前风格使用GP后退 |
| 4G guard performance for GP and red shield | GP和红盾的防御性能变为4G |
| Shield thrust X+A to yellow shield | 盾突X+A接黄盾0x44 |
| Guard recoil (small/medium) X+A to yellow shield | 防御小退/中退后X+A接黄盾0x4B |
| Additional AED follow-ups | 大解额外派生 |
| J guard X+A to yellow shield | J防成功后X+A接黄盾0x44，独立全局开关 |

## SAED保持全局开关

```ini
[saed]
enabled = true
effect_gear = 2
```

按用户最后确认，SAED不再按风格拆分。开启时原21处补丁与三个瓶爆钩子同时启用；关闭时用原游戏/SH逻辑，但HDP仍始终81条。

- 第三hit：地面调用0x50、空中调用0x23；后续帧常量108改109；不改原0x4F调用。
- 瓶伤：固定原金手指系数1.35，精确字节0x3FACCCCC。
- 榴弹：首爆等待0、前方270；之后间隔9、每爆前进100。
- 强属性：首爆等待46、前方300，后续1/2/3/4点扇形；相对时间0/18/20/38/40/42/58/60/62/64。
- 逐爆探测地面高度，特效与判定共用坐标；支持1..10瓶，最后等待9再回收。
- `effect_gear`允许1..4，默认2；这是资源档位，不是effectId或真实瓶数。

完整时序见[SAED](SAED.md)。默认模板已为每个开关列出具体涉及的动作、字段和补丁。

## Presentation

原有音效、GU盾牌闪电和任务时序设置保持原含义。六项4G视觉独立启停：

```ini
[effects]
enabled = true

[4g_effects]
aed_explosion = true
saed_explosion = true
phial_attachment = true
ed_blast = true
aed_blast = true
saed_blast = false
```

- 前两项只追加独立提取的005/103后段；不再整段替换原GU闪电，不再提供旧的两项完整序列时长。
- 后四项只改真实瓶附加/瓶爆的资源；地面、水面与元素分别筛选，不改伤害、瓶数及时间表。
- `saed_blast`因4G107观感较糊默认关闭，回退GU超解瓶爆；其余五项默认开启，103砸地和Moveset超解不受它控制。
- 菜单顺序为4G特效、GU lightning、Sound；action/burst条目按用途显示在GU或Sound下，仍读写原任务编号。
- 旧`use_4g_sequences/aed_4g_duration/saed_4g_duration`读取兼容但无效，保存移除。原GU action/burst时长仍保留。

此前错误的独立002命中/104砸地功能未恢复；002仅作为本轮要求的瓶附加，首爆时立即结束。旧INI的[impacts]两键仅读取兼容，保存移除。005使用真实命中/PSL地面请求，103仅在有瓶超解砸下创建。后段使用独立三资源overlay，29项完整预览包不改，均内嵌NRO。菜单保存到同一存档根目录INI，重启后应用。详见[事件及停止修正](2026-10-24_逆向-005103事件与002首爆停止-report.md)。
