//! Built-in term glossary (no LLM required).
//! Edit this file to add more entries; keys are matched case-insensitively.

use std::collections::HashMap;
use std::sync::LazyLock;

/// (canonical display name, explanation)
pub static GLOSSARY: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> =
    LazyLock::new(|| {
        HashMap::from([
            // --- Acronyms (tech-focused) ---
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
            ("iot", ("IoT", "Internet of Things，物联网")),
            ("vpn", ("VPN", "Virtual Private Network，虚拟专用网络")),
            ("dns", ("DNS", "Domain Name System，域名系统")),
            ("cdn", ("CDN", "Content Delivery Network，内容分发网络")),
            ("nlp", ("NLP", "Natural Language Processing，自然语言处理")),
            ("cv", ("CV", "Computer Vision，计算机视觉")),
            ("asr", ("ASR", "Automatic Speech Recognition，自动语音识别")),
            ("rlhf", ("RLHF", "Reinforcement Learning from Human Feedback，人类反馈强化学习")),
            ("moe", ("MoE", "Mixture of Experts，混合专家模型，用稀疏激活扩展模型容量")),
            ("lora", ("LoRA", "Low-Rank Adaptation，低秩适配，轻量微调大模型的常见方法")),
            ("sft", ("SFT", "Supervised Fine-Tuning，监督微调")),
            ("dpo", ("DPO", "Direct Preference Optimization，直接偏好优化，对齐模型输出的训练方法")),
            ("ppo", ("PPO", "Proximal Policy Optimization，近端策略优化，RLHF 中常用的强化学习算法")),
            ("mcp", ("MCP", "Model Context Protocol，模型上下文协议，用于连接 LLM 与外部工具/数据源")),
            ("vlm", ("VLM", "Vision-Language Model，视觉-语言多模态模型")),
            ("kv", ("KV cache", "Key-Value 缓存，推理时缓存注意力键值以加速自回归生成")),
            ("fp16", ("FP16", "16 位浮点精度，常用于 GPU 推理/训练以节省显存")),
            ("bf16", ("BF16", "Brain Float 16，深度学习常用数值格式，动态范围优于 FP16")),
            ("int8", ("INT8", "8 位整数量化，压缩模型权重以加速推理、降低内存占用")),
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
                "tokenizer",
                (
                    "tokenizer",
                    "分词器：把原始文本切分为 token 序列，不同模型的分词策略会影响上下文长度与成本。",
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
                "quantization",
                (
                    "quantization",
                    "量化：降低模型权重/激活精度（如 INT8/FP16）以加速推理并减少显存占用。",
                ),
            ),
            (
                "distillation",
                (
                    "distillation",
                    "知识蒸馏：用小模型模仿大模型输出，在保持部分能力的同时降低部署成本。",
                ),
            ),
            (
                "reranker",
                (
                    "reranker",
                    "重排序模型：对初检召回的文档/片段再打分排序，提升 RAG 检索精度。",
                ),
            ),
            (
                "reranking",
                (
                    "reranking",
                    "重排序：对检索候选结果二次打分，常用于 RAG 流水线末段。",
                ),
            ),
            (
                "agent",
                (
                    "agent",
                    "智能体：能调用工具、规划步骤、与环境交互的 LLM 应用形态。",
                ),
            ),
            (
                "diffusion",
                (
                    "diffusion",
                    "扩散模型：通过逐步去噪生成图像/音频等，Stable Diffusion 即属此类。",
                ),
            ),
            (
                "multimodal",
                (
                    "multimodal",
                    "多模态：同时处理文本、图像、音频等多种输入/输出的模型能力。",
                ),
            ),
            (
                "pretraining",
                (
                    "pretraining",
                    "预训练：在大规模无标注数据上训练基础模型，后续再微调下游任务。",
                ),
            ),
            (
                "backpropagation",
                (
                    "backpropagation",
                    "反向传播：通过链式法则计算梯度并更新神经网络权重的核心训练算法。",
                ),
            ),
            (
                "softmax",
                (
                    "softmax",
                    "Softmax：把 logits 转为概率分布，常用于分类与注意力权重归一化。",
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
            (
                "kubernetes",
                (
                    "Kubernetes",
                    "容器编排平台，用于部署、扩缩容与管理微服务与 AI 推理服务。",
                ),
            ),
            (
                "docker",
                (
                    "Docker",
                    "容器化平台，把应用与依赖打包为可移植镜像，便于部署 LLM 服务。",
                ),
            ),
            (
                "grpc",
                (
                    "gRPC",
                    "高性能 RPC 框架，常用 Protocol Buffers 序列化，适合服务间低延迟通信。",
                ),
            ),
            (
                "pytorch",
                (
                    "PyTorch",
                    "主流深度学习框架，动态计算图，广泛用于 LLM 训练与推理。",
                ),
            ),
            (
                "tensorflow",
                (
                    "TensorFlow",
                    "Google 开源深度学习框架，支持训练部署与 TPU 加速。",
                ),
            ),
            (
                "langchain",
                (
                    "LangChain",
                    "LLM 应用开发框架，封装 RAG、Agent、工具调用等常见模式。",
                ),
            ),
            (
                "llamaindex",
                (
                    "LlamaIndex",
                    "面向数据连接的 LLM 框架，侧重索引、检索与 RAG 流水线。",
                ),
            ),
            (
                "chromadb",
                (
                    "ChromaDB",
                    "开源向量数据库，常用于 embedding 存储与语义检索。",
                ),
            ),
            (
                "weaviate",
                (
                    "Weaviate",
                    "开源向量搜索引擎，支持混合检索与 GraphQL 查询。",
                ),
            ),
            (
                "pinecone",
                (
                    "Pinecone",
                    "托管向量数据库服务，用于大规模 embedding 检索。",
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
