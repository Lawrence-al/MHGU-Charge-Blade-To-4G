# SAED 瓶爆与 HDP 扩容范围

- auth：用户要求实现两项 SAED NRO 修改；榴弹间隔已纠正为 9，特效档位为 2。本次选择按 4G 普通地面超解首爆基准还原，并保持后续相对时间和位置；最新明确盾突有红盾就播放，不等待命中。0x4F 文件数据由用户处理。之前已授权编译并复制 NRO 到游戏目录。
- in_scope：读取本地 MH4G／MHGU 二进制及已有反编译导出；更改 SAED 首爆倒计时和位置基准、Presentation 盾突起手特效、相应验证与文档；编译并部署这两个 NRO。
- out_of_scope：修改 SH／HDP 本地资源、修改瓶伤四条补丁、改变空中第三下 0x23、改动 Moveset NRO、git 提交／分支。
- network_profile：offline；未联网、未向其它人发送文件或消息。
- inputs：MHGU 1.4.0 原版 main／main.c；MH4G 1.2 ExeFS.elf／ExeFS.elf.c；项目当前 asset HDP。
- verification：离线 ARM 指令解释执行、实际 Rust 纯逻辑及 HDP 数据验证、编译／二进制核对；无游戏战斗运行证据。
