# Moveset 中的 Hitdata 功能

已整合进 `cb_moveset_to_4g.nro`；配置为 [INI](Configuration.md) 的 `[hitdata] enabled`。源码位于 `Moveset-to-4G/src/hitdata/`。

构建时嵌入当前 `assets/romfs/nativeNX/arc/player/quest/w14.arc/hit/pl_we14_hitdata.4E2EF008`。开启时将 0x00..0x4E 的 79 条记录复制成 asset，0x4F 写回 GU 原版完整快照。改 asset 后重编 Moveset 即可更新；运行时不读取项目路径。关闭时不复制这些记录，也不恢复外部资源文件自己的修改。

2026-10-24用户已将0x15/1F/21/44/4A的 `HunterArtFlag（record+0x0E）` 从1关闭为0，随重编带入NRO。这是旧狩技命中外观标记；转换005命中特效由Presentation真实瓶爆事件创建，与它独立。`HunterArtGen` 位于+0x26，不是此次关闭的字节。

记录长 56 字节，头长 16。asset 范围 [0x10,0x1158)；0x4F 范围 [0x1158,0x1190)，原版出现延迟106、持续6、动作值55。它是原版地面无瓶超解variant3特定分支的第二组HDP：该分支先读0x4E到slot0，再读0x4F到slot1，其它分支使用0x19/0x1A。它不是新增第三hit。

早期方案借用了0x4F，把它改成0/8/100，并把原调用011DB014改读0x1A。追加0x50后，这条替换补丁已经移除；当前Hitdata开启时写回原版0x4F完整快照，是撤回早期占用。关闭Hitdata则保留实际加载文件自己的0x4F内容，不替用户恢复外部ARC。

新增0x50是第三hit独立快照：固定延迟0、持续8、动作值100，始终注册，不从当前0x4F取值。地面超解是否主动调用它由 `[saed] enabled` 控制；空战第三hit使用原有0x23。这里的HDP条目0x4F与黄盾动作ID0x4F不是同一种编号。

## 单一加载链

原版资源管理器 → 原 HDP load 入口 main+0x00270C18 → 原函数调用一次 → 80条扩为81条（已有81条不重复扩）→ 可选复制asset／恢复0x4F → 写入公共0x50 → 返回游戏。

不再修改 main+0x01747CCC 的加载虚表，也不依赖两个 NRO 的先后顺序。Hitdata 与 SAED 全关时不挂加载钩子；仅 Hitdata 开启时仍为 80 条；SAED 开启时追加第 81 条。已为 81 条的输入不再次扩容。扩容沿用原 MtAllocator，释放旧缓冲区并同步更新 resource+0x64、+0x68，重载／析构仍由原游戏管理。

资源路径只匹配盾斧 pl_we14_hitdata，支持原两种内部路径、大小写、正反斜杠及 .hdp 后缀。其它武器、错误条数不修改；原加载失败不进入修改流程，返回值保留。

## 验证

`node tests/verify-hitdata.js "D:/Work/dump-classicMH/MHGU dump"` 执行实际统一加载器和入口回调的主机适配版本。864 组场景覆盖两种输入文件、Hitdata／SAED 开关、插件总开关、成功／失败／不同返回值、资源筛选、条数、重载、分配器释放和两个资源指针。0x00..0x4E 对照完整 asset，0x4F 对照完整 GU 原版，0x50 对照固定第三 hit 快照。编译后的 Moveset NRO 也包含两份完整记录。

离线验证不代替游戏战斗测试。原始数据和字段依据见 [HDP 与 J 防报告](2026-10-03_逆向-Hitdata与J防派生-report.md)。
