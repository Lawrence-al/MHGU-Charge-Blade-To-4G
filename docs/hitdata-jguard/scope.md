# HDP 与 J 防派生范围

- auth：用户要求第 6 项新增独立 NRO 复现当前 asset HDP；随后明确要求 0x4F 使用原版，不再使用 asset 的修改。第 7 项 J 防成功后 X+A 改为动作 0x44，参考 w14_m3.arc，避免整体替换共享动作。沿用已授权的编译和复制 NRO 到游戏目录。
- in_scope：读取本地原版 main、IDA C 导出、原版 w14_m3.arc 和当前 asset；新增 Hitdata 子项目；修改 Moveset 的特定派生；提取 HDP 共用路径匹配；更新构建、验证与说明，部署产物。
- out_of_scope：编辑 asset／dump 原文件；修改瓶伤、三 hit、瓶爆间隔／位置和音效／特效；复现 w14_m3.arc 中其它派生表的更改；提交、分支或删除文件。
- network_profile：分析和验证仅使用本地输入；新 Cargo 包首次构建访问 crates.io 解析依赖，沙盒网络失败后通过工具审批在沙盒外完成构建。未向他人发送项目数据。
- verification：编译后 ARM 桥接、实际 Rust 数据修改、加载回调主机适配、原值检查、文件散列及安装范围核对；尚无游戏战斗结果。
