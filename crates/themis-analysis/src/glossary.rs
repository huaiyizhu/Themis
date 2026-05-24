//! Built-in term glossary (no LLM required).
//! Edit this file to add more entries; keys are matched case-insensitively.

use std::collections::HashMap;
use std::sync::LazyLock;

/// (canonical display name, explanation)
pub static GLOSSARY: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> =
    LazyLock::new(|| {
        HashMap::from([
            // --- Acronyms ---
            ("nba", ("NBA", "National Basketball Association，美国职业篮球联赛")),
            ("ai", ("AI", "Artificial Intelligence，人工智能")),
            ("ml", ("ML", "Machine Learning，机器学习")),
            ("llm", ("LLM", "Large Language Model，大语言模型")),
            ("rag", ("RAG", "Retrieval-Augmented Generation，检索增强生成")),
            ("gpu", ("GPU", "Graphics Processing Unit，图形处理器，常用于深度学习加速")),
            ("cpu", ("CPU", "Central Processing Unit，中央处理器")),
            ("api", ("API", "Application Programming Interface，应用程序接口")),
            ("http", ("HTTP", "Hypertext Transfer Protocol，超文本传输协议")),
            ("rest", ("REST", "Representational State Transfer，一种 Web API 架构风格")),
            ("sql", ("SQL", "Structured Query Language，结构化查询语言")),
            ("json", ("JSON", "JavaScript Object Notation，常用数据交换格式")),
            ("ocr", ("OCR", "Optical Character Recognition，光学字符识别")),
            ("stt", ("STT", "Speech-to-Text，语音转文字")),
            ("tts", ("TTS", "Text-to-Speech，文字转语音")),
            ("aws", ("AWS", "Amazon Web Services，亚马逊云计算平台")),
            ("azure", ("Azure", "Microsoft Azure，微软云计算平台")),
            ("gpt", ("GPT", "Generative Pre-trained Transformer，生成式预训练 Transformer")),
            ("ui", ("UI", "User Interface，用户界面")),
            ("ux", ("UX", "User Experience，用户体验")),
            ("iot", ("IoT", "Internet of Things，物联网")),
            ("vpn", ("VPN", "Virtual Private Network，虚拟专用网络")),
            ("dns", ("DNS", "Domain Name System，域名系统")),
            ("cdn", ("CDN", "Content Delivery Network，内容分发网络")),
            ("kpi", ("KPI", "Key Performance Indicator，关键绩效指标")),
            ("roi", ("ROI", "Return on Investment，投资回报率")),
            ("ceo", ("CEO", "Chief Executive Officer，首席执行官")),
            ("cto", ("CTO", "Chief Technology Officer，首席技术官")),
            // --- AI / ML terms (often spoken lowercase) ---
            (
                "embedding",
                (
                    "embedding",
                    "嵌入向量：把文本/图像等映射到高维向量空间，用于语义搜索、相似度计算与 RAG 检索。",
                ),
            ),
            (
                "embeddings",
                (
                    "embeddings",
                    "嵌入向量（复数），多个对象在向量空间中的表示，用于检索与聚类。",
                ),
            ),
            (
                "transformer",
                (
                    "Transformer",
                    "基于自注意力机制的神经网络架构，是现代 LLM 的基础。",
                ),
            ),
            (
                "attention",
                (
                    "attention",
                    "注意力机制：模型为不同 token 分配权重，以捕捉长距离依赖。",
                ),
            ),
            (
                "token",
                (
                    "token",
                    "模型处理文本的最小单位（可能是字、词或子词），影响计费与上下文长度。",
                ),
            ),
            (
                "tokens",
                (
                    "tokens",
                    "token 的复数形式，常用来衡量输入/输出长度。",
                ),
            ),
            (
                "vector",
                (
                    "vector",
                    "向量：一组数值，用于表示 embedding 或特征，可做相似度检索。",
                ),
            ),
            (
                "fine-tuning",
                (
                    "fine-tuning",
                    "微调：在预训练模型上继续训练，使其适应特定任务或领域数据。",
                ),
            ),
            (
                "finetuning",
                (
                    "fine-tuning",
                    "微调：在预训练模型上用领域数据继续训练。",
                ),
            ),
            (
                "prompt",
                (
                    "prompt",
                    "提示词：发给模型的输入指令或上下文，影响回答风格与质量。",
                ),
            ),
            (
                "inference",
                (
                    "inference",
                    "推理：模型在训练完成后对新输入进行预测/生成的阶段。",
                ),
            ),
            (
                "hallucination",
                (
                    "hallucination",
                    "幻觉：模型生成看似合理但事实错误的内容。",
                ),
            ),
            (
                "cosmos",
                (
                    "Cosmos DB",
                    "Azure Cosmos DB，微软的全球分布式多模型数据库服务。",
                ),
            ),
            (
                "cosmosdb",
                (
                    "Cosmos DB",
                    "Azure Cosmos DB，微软的全球分布式多模型数据库服务。",
                ),
            ),
        ])
    });

pub fn lookup(term: &str) -> Option<(&'static str, &'static str)> {
    let key = term.trim().to_lowercase();
    if key.is_empty() {
        return None;
    }
    GLOSSARY.get(key.as_str()).copied()
}
