# Contribute Guide

[English](#english) | [中文](#中文)

---

## English

Thank you for contributing!

### 📌 Where to Add Aliases

All player aliases are defined in the following file:

👉 [src/main.rs](https://github.com/SilverLi0x10/stdscore-GUI/blob/main/src/main.rs)

Inside this file, you will find a static mapping table like this:

```rust
/\*
-   Replacement for name in the table
-   (name -> replacement)
-   where name is LOWERCASE
    \*/
    static REPLACE_NAME: phf::Map<&str, &str> = phf_map!(
    "cqyc-xxx" => "CQYC-目标人名",
    // Add new aliases here
    );
```

### 🛠 How to Contribute

1. Open the file: `src/main.rs`
2. Inside the `phf_map!` macro, add your new alias in the format:

```rust
"lowercase-name" => "DisplayName",
```

-   **Left side** : lowercase letters, numbers, or dashes (unique ID)
-   **Right side** : display name (can include Chinese, English, etc.)

3. Save your changes and submit a Pull Request.

---

## 中文

感谢你的贡献！

### 📌 在哪里添加别名

所有选手别名都定义在以下文件中：

👉 [src/main.rs](https://github.com/SilverLi0x10/stdscore-GUI/blob/main/src/main.rs)

在该文件中，你会看到一个静态映射表，例如：

```rust
/\*
-   Replacement for name in the table
-   (name -> replacement)
-   where name is LOWERCASE
    \*/
    static REPLACE_NAME: phf::Map<&str, &str> = phf_map!(
    "cqyc-xxx" => "CQYC-目标人名",
    // 在这里添加新的别名
    );
```

### 🛠 如何贡献

1. 打开文件：`src/main.rs`
2. 在 `phf_map!` 宏中，按照以下格式添加新的别名：

```rust
"lowercase-name" => "展示名",
```

-   **左边** ：必须是小写字母、数字或短横线（唯一 ID）
-   **右边** ：展示用的别名（可以包含中文、英文等）

3. 保存修改并提交 Pull Request。
