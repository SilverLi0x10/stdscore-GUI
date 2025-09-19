# 📊 std score calculator

A desktop GUI tool built with `eframe` and `egui` to parse HTML files containing score tables, normalize scores based on the highest non-"std" entry, and display a sortable comparison table. Supports drag-and-drop, Chinese font rendering, and system-aware dark mode.

---

## 🌐 Language / 语言切换

-   [English](#english)
-   [中文](#中文)

---

## English

### 🧩 Features

-   Drag and drop multiple HTML files containing score tables
-   Automatically parses and extracts names and scores
-   Calculates standardized scores:  
    `std_score = raw_score / highest_normal_score * 100`
-   Displays sortable table with:
    -   Name
    -   Average standardized score
    -   Per-file standardized and raw scores
-   Supports Chinese fonts (Noto Sans SC or Microsoft YaHei)
-   **Dark mode support (follows system preference by default)**

### 📦 Dependencies

-   `eframe`
-   `egui`
-   `egui_extras`
-   `scraper`
-   `regex`
-   `serde`
-   `anyhow`
-   `rfd` (for file dialog)

### 🚀 How to Run

```bash
cargo run --release
```

### 📁 HTML Format Requirement

-   The score table must be inside the **third `<p>` tag** under `<body>`
-   Each row must contain at least:
    -   Rank
    -   Name (in second column, optionally wrapped in `<a>`)
    -   Score (in third column)

### 🖼 Font Setup

Automatically attempts to load:

1. `NotoSansSC-Regular.ttf` from system fonts
2. Fallback to `msyh.ttc` (Microsoft YaHei)

### 🛠 Future Improvements

-   Customizable parsing rules
-   Export to CSV

### 📬 Feedback

Feel free to open issues or submit pull requests. Contributions are welcome!
👉 [Contributing Guide](https://github.com/SilverLi0x10/stdscore-GUI/blob/main/CONTRIBUTING.md)

---

## 中文

### 🧩 功能介绍

-   拖拽多个包含成绩表的 HTML 文件
-   自动解析姓名与分数
-   计算标准分数：  
    `标准分 = 原始分 / 文件中最高正常分 * 100`
-   显示可排序的对比表格，包括：
    -   姓名
    -   平均标准分
    -   每个文件的标准分与原始分
-   支持中文字体渲染（优先使用 Noto Sans SC，其次为微软雅黑）
-   **支持暗黑模式（默认跟随系统设置）**

### 📦 依赖库

-   `eframe`
-   `egui`
-   `egui_extras`
-   `scraper`
-   `regex`
-   `serde`
-   `anyhow`
-   `rfd`（用于文件选择对话框）

### 🚀 运行方式

```bash
cargo run --release
```

### 📁 HTML 文件格式要求

-   成绩表必须位于 `<body>` 下第三个 `<p>` 标签中
-   每行至少包含：
    -   排名
    -   姓名（第二列，可包含 `<a>` 标签）
    -   分数（第三列）

### 🖼 字体设置

程序会自动尝试加载：

1. 系统字体中的 `NotoSansSC-Regular.ttf`
2. 若无则回退至 `msyh.ttc`（微软雅黑）

### 🛠 后续计划

-   支持自定义解析规则
-   导出 CSV 文件

### 📬 反馈

欢迎提交 Issue 或 Pull Request，一起改进本项目！

👉 [贡献指南](https://github.com/SilverLi0x10/stdscore-GUI/blob/main/CONTRIBUTING.md)
