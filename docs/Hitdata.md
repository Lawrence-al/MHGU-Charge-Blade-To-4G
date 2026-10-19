# Moveset 中的 Hitdata 功能

已整合进 `cb_moveset_to_4g.nro`；配置为 [INI](Configuration.md) 的 `[hitdata] enabled`。源码位于 `Moveset-to-4G/src/hitdata/`。

构建时嵌入当前 `assets/romfs/nativeNX/arc/player/quest/w14.arc/hit/pl_we14_hitdata.4E2EF008`。开启时将 0x00..0x4E 的 79 条记录复制成 asset，0x4F 写回 GU 原版完整快照。改 asset 后重编 Moveset 即可更新；运行时不读取项目路径。关闭时不复制这些记录，也不恢复外部资源文件自己的修改。

记录长 56 字节，头长 16。asset 范围 [0x10,0x1158)；0x4F 范围 [0x1158,0x1190)，原版出现延迟 106、持续 6、动作值 55。即使加载改版 HDP，开启时也恢复完整原版 0x4F。新增 0x50 是第三 hit 的独立快照：固定出现延迟 0、持续 8、动作值 100，由 `[saed] enabled` 控制。

## 单一加载链

原版资源管理器 → 原 HDP load 入口 main+0x00270C18 → 原函数调用一次 → 可选 80 扩为 81 条 → 可选复制 asset／恢复 0x4F → 可选设置 0x50 → 返回游戏。

不再修改 main+0x01747CCC 的加载虚表，也不依赖两个 NRO 的先后顺序。Hitdata 与 SAED 全关时不挂加载钩子；仅 Hitdata 开启时仍为 80 条；SAED 开启时追加第 81 条。已为 81 条的输入不再次扩容。扩容沿用原 MtAllocator，释放旧缓冲区并同步更新 resource+0x64、+0x68，重载／析构仍由原游戏管理。

资源路径只匹配盾斧 pl_we14_hitdata，支持原两种内部路径、大小写、正反斜杠及 .hdp 后缀。其它武器、错误条数不修改；原加载失败不进入修改流程，返回值保留。

## 验证

`node tests/verify-hitdata.js "D:/Work/dump-classicMH/MHGU dump"` 执行实际统一加载器和入口回调的主机适配版本。864 组场景覆盖两种输入文件、Hitdata／SAED 开关、插件总开关、成功／失败／不同返回值、资源筛选、条数、重载、分配器释放和两个资源指针。0x00..0x4E 对照完整 asset，0x4F 对照完整 GU 原版，0x50 对照固定第三 hit 快照。编译后的 Moveset NRO 也包含两份完整记录。

离线验证不代替游戏战斗测试。原始数据和字段依据见 [HDP 与 J 防报告](2026-10-03_逆向-Hitdata与J防派生-report.md)。
