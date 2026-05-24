//! Built-in term glossary (no LLM required).
//! Edit this file to add more entries; keys are matched case-insensitively.
//! Explanations: 中文优先，简洁说明用途、作用、用法与例子。

use std::collections::HashMap;
use std::sync::LazyLock;

/// (canonical display name, explanation)
pub static GLOSSARY: LazyLock<HashMap<&'static str, (&'static str, &'static str)>> =
    LazyLock::new(|| {
        HashMap::from([
            // --- Acronyms (tech-focused) ---
            (
                "ai",
                (
                    "AI",
                    "人工智能：让机器完成理解、推理、生成等智能任务。用途：对话助手、图像识别、推荐等。例：ChatGPT 即 AI 应用。",
                ),
            ),
            (
                "ml",
                (
                    "ML",
                    "机器学习：从数据中自动学习规律，而非硬编码规则。用途：分类、预测、聚类。例：用历史邮件训练垃圾邮件过滤器。",
                ),
            ),
            (
                "llm",
                (
                    "LLM",
                    "大语言模型：在海量文本上预训练，可理解并生成自然语言。用途：问答、写作、代码生成、Agent 大脑。例：GPT-4、Claude。",
                ),
            ),
            (
                "rag",
                (
                    "RAG",
                    "检索增强生成：先查知识库/文档，再让模型基于检索结果回答。作用：减少幻觉、接入私有资料。例：企业 wiki 问答先搜文档再生成答案。",
                ),
            ),
            (
                "gpu",
                (
                    "GPU",
                    "图形处理器，并行算力强。用途：深度学习训练与推理加速。例：跑 LLM 推理通常需要 GPU 显存。",
                ),
            ),
            (
                "cpu",
                (
                    "CPU",
                    "中央处理器，通用计算核心。用途：系统调度、轻量推理、数据预处理；大模型主力算力多在 GPU。",
                ),
            ),
            (
                "api",
                (
                    "API",
                    "应用程序接口：程序之间约定的调用方式。用法：发 HTTP 请求获取数据或调用服务。例：调用 OpenAI API 发 prompt 拿回复。",
                ),
            ),
            (
                "http",
                (
                    "HTTP",
                    "超文本传输协议，Web 通信基础。用途：浏览器访问网页、REST API 传 JSON。例：POST /chat/completions 即 HTTP 请求。",
                ),
            ),
            (
                "rest",
                (
                    "REST",
                    "一种 Web API 设计风格，用 URL 表资源、HTTP 动词表操作。用法：GET 查询、POST 创建。例：GET /users/1 取用户。",
                ),
            ),
            (
                "sql",
                (
                    "SQL",
                    "结构化查询语言，操作关系型数据库。用途：查改表数据。例：SELECT * FROM orders WHERE status='paid'。",
                ),
            ),
            (
                "json",
                (
                    "JSON",
                    "轻量数据交换格式，键值对+数组。用途：API 请求/响应、配置文件。例：{\"keywords\":[\"RAG\"]}。",
                ),
            ),
            (
                "ocr",
                (
                    "OCR",
                    "光学字符识别：从图片/PDF 提取文字。用途：票据录入、扫描文档数字化。例：拍照发票自动读出金额。",
                ),
            ),
            (
                "stt",
                (
                    "STT",
                    "语音转文字。用途：会议听写、字幕、语音助手输入。例：Themis 即 STT 流水线。",
                ),
            ),
            (
                "tts",
                (
                    "TTS",
                    "文字转语音。用途：有声读物、导航播报、语音助手回复。例：输入文本输出 mp3 语音。",
                ),
            ),
            (
                "aws",
                (
                    "AWS",
                    "亚马逊云计算平台。用途：虚拟机、对象存储、托管数据库与 AI 服务部署。",
                ),
            ),
            (
                "azure",
                (
                    "Azure",
                    "微软云计算平台。用途：Speech、OpenAI、Cosmos DB 等；Themis 常用 Azure Speech 做听写。",
                ),
            ),
            (
                "gpt",
                (
                    "GPT",
                    "生成式预训练 Transformer 系列模型。用途：文本生成与理解。例：GPT-4o 用于对话与代码。",
                ),
            ),
            (
                "iot",
                (
                    "IoT",
                    "物联网：设备联网采集与上报数据。用途：智能家居、工业传感。例：温湿度传感器上报云端。",
                ),
            ),
            (
                "vpn",
                (
                    "VPN",
                    "虚拟专用网络，加密隧道访问内网或隐藏 IP。用途：远程办公连公司网、跨区访问。",
                ),
            ),
            (
                "dns",
                (
                    "DNS",
                    "域名系统：把域名解析为 IP。用途：浏览器输入 github.com 时找到服务器地址。",
                ),
            ),
            (
                "cdn",
                (
                    "CDN",
                    "内容分发网络：把静态资源缓存到边缘节点。作用：加速网页/视频加载，减轻源站压力。",
                ),
            ),
            (
                "nlp",
                (
                    "NLP",
                    "自然语言处理：让机器理解/生成人类语言。用途：翻译、情感分析、问答、Themis 字幕分析。",
                ),
            ),
            (
                "cv",
                (
                    "CV",
                    "计算机视觉：从图像/视频理解内容。用途：人脸识别、缺陷检测、VLM 看图问答。",
                ),
            ),
            (
                "asr",
                (
                    "ASR",
                    "自动语音识别，即 STT。用途：实时字幕、语音命令。例：Azure Speech 做流式 ASR。",
                ),
            ),
            (
                "rlhf",
                (
                    "RLHF",
                    "人类反馈强化学习：用人排序/打分指导模型对齐人类偏好。用途：让回答更安全、更有帮助。例：ChatGPT 对齐训练常用 RLHF。",
                ),
            ),
            (
                "moe",
                (
                    "MoE",
                    "混合专家模型：多组「专家」子网络，每次只激活一部分。作用：扩大容量同时控制推理成本。例：Mixtral 即 MoE 架构。",
                ),
            ),
            (
                "lora",
                (
                    "LoRA",
                    "低秩适配：只训练少量附加参数即可微调大模型。用途：省显存、快速定制领域模型。例：用 LoRA 微调医疗问答模型。",
                ),
            ),
            (
                "sft",
                (
                    "SFT",
                    "监督微调：用标注好的输入-输出对继续训练模型。用途：教模型按格式答题、遵循指令。例：指令数据集微调 LLM。",
                ),
            ),
            (
                "dpo",
                (
                    "DPO",
                    "直接偏好优化：用「更好/更差」回答对比直接优化模型，替代部分 RLHF 流程。用途：对齐输出风格与质量。",
                ),
            ),
            (
                "ppo",
                (
                    "PPO",
                    "近端策略优化：强化学习算法，稳定更新策略。用途：RLHF 第二阶段训练策略模型。",
                ),
            ),
            (
                "mcp",
                (
                    "MCP",
                    "模型上下文协议：标准化 LLM 与外部工具/数据源连接。用法：Agent 通过 MCP 读文件、调 API。例：Cursor 工具集成。",
                ),
            ),
            (
                "vlm",
                (
                    "VLM",
                    "视觉-语言模型：同时理解图像与文本。用途：看图问答、文档 OCR+理解。例：上传截图问「这页代码做什么」。",
                ),
            ),
            (
                "kv",
                (
                    "KV cache",
                    "推理时缓存已算过的 Key/Value，避免重复计算。作用：显著加速长文本自回归生成。用法：开启后首 token 慢、后续 token 快。",
                ),
            ),
            (
                "fp16",
                (
                    "FP16",
                    "16 位浮点：省显存、提速。用途：GPU 训练/推理常用精度。注意：极端值可能溢出，需与 loss scaling 等配合。",
                ),
            ),
            (
                "bf16",
                (
                    "BF16",
                    "Brain Float 16：动态范围大、训练更稳。用途：A100/H100 等大模型训练默认格式之一。",
                ),
            ),
            (
                "int8",
                (
                    "INT8",
                    "8 位整数量化：压缩模型权重。作用：推理更快、占内存更少，略损精度。例：边缘设备部署小模型。",
                ),
            ),
            // --- AI / ML terms (often spoken lowercase) ---
            (
                "embedding",
                (
                    "embedding",
                    "嵌入向量：把文本/图像映射为数值向量。用途：语义搜索、相似度计算、RAG 检索。例：「机器学习」与「ML」向量相近。",
                ),
            ),
            (
                "embeddings",
                (
                    "embeddings",
                    "多个对象的向量表示集合。用途：批量检索、聚类分析。用法：文档切块后逐块算 embedding 存入向量库。",
                ),
            ),
            (
                "transformer",
                (
                    "Transformer",
                    "基于自注意力的神经网络架构。作用：并行处理序列、捕捉长距离依赖。用途：现代 LLM 的基础结构。",
                ),
            ),
            (
                "attention",
                (
                    "attention",
                    "注意力机制：为每个词分配不同权重，聚焦相关信息。作用：理解上下文关联。例：翻译「它」时关注前文指代对象。",
                ),
            ),
            (
                "token",
                (
                    "token",
                    "模型处理文本的最小单位（字/词/子词）。用途：计费和上下文长度限制。例：英文 \"hello\" 可能为 1 token。",
                ),
            ),
            (
                "tokens",
                (
                    "tokens",
                    "token 的数量单位。用途：衡量 prompt 长度、API 计费。用法：上下文 128k tokens 即约可放更长文档。",
                ),
            ),
            (
                "tokenizer",
                (
                    "tokenizer",
                    "分词器：把原文切成 token 序列。作用：决定模型如何「读」文本。注意：不同模型 tokenizer 不通用，影响长度与成本。",
                ),
            ),
            (
                "vector",
                (
                    "vector",
                    "向量：一组数值，表示对象特征或语义。用途：算余弦相似度做检索。例：query 向量与文档 embedding 比相似度。",
                ),
            ),
            (
                "fine-tuning",
                (
                    "fine-tuning",
                    "微调：在预训练模型上用领域数据继续训练。用途：适配医疗、法律等垂直场景。例：用公司内部 FAQ 微调客服模型。",
                ),
            ),
            (
                "finetuning",
                (
                    "fine-tuning",
                    "微调：在预训练模型上用领域数据继续训练。用途：适配医疗、法律等垂直场景。例：用公司内部 FAQ 微调客服模型。",
                ),
            ),
            (
                "prompt",
                (
                    "prompt",
                    "提示词：发给模型的指令与上下文。作用：控制回答风格、格式与任务。例：「用三句话总结以下文章：…」。",
                ),
            ),
            (
                "inference",
                (
                    "inference",
                    "推理：训练完成后对新输入做预测/生成。用途：线上问答、批处理生成。与训练相对，通常更关注延迟与吞吐。",
                ),
            ),
            (
                "hallucination",
                (
                    "hallucination",
                    "幻觉：模型生成看似合理但事实错误的内容。应对：RAG 附证据、引用来源、降低 temperature。例：编造不存在的论文标题。",
                ),
            ),
            (
                "quantization",
                (
                    "quantization",
                    "量化：降低权重/激活数值精度（如 INT8/FP16）。作用：加速推理、省显存。用法：部署前用工具对模型做量化转换。",
                ),
            ),
            (
                "distillation",
                (
                    "distillation",
                    "知识蒸馏：让小模型模仿大模型输出。作用：在可接受精度损失下降低部署成本。例：大模型打标签训练小模型。",
                ),
            ),
            (
                "reranker",
                (
                    "reranker",
                    "重排序模型：对初检召回结果再精细打分排序。用途：提升 RAG 检索 Top 结果质量。用法：向量检索取 Top50 → reranker 取 Top5。",
                ),
            ),
            (
                "reranking",
                (
                    "reranking",
                    "重排序步骤：对候选文档二次打分。作用：弥补向量检索粗排不足。常见于 RAG 流水线检索后段。",
                ),
            ),
            (
                "agent",
                (
                    "agent",
                    "智能体：LLM + 规划 + 工具调用，可多步完成任务。用途：查数据库、写代码、订机票。例：「帮我分析这份 CSV 并画图」。",
                ),
            ),
            (
                "diffusion",
                (
                    "diffusion",
                    "扩散模型：从噪声逐步去噪生成内容。用途：文生图、文生音频。例：Stable Diffusion 输入 prompt 出图。",
                ),
            ),
            (
                "multimodal",
                (
                    "multimodal",
                    "多模态：同时处理文本、图像、音频等。用途：看图说话、视频理解。例：GPT-4o 可语音+图像输入。",
                ),
            ),
            (
                "pretraining",
                (
                    "pretraining",
                    "预训练：在大规模无标注语料上训练基础能力。作用：学会语言/general 知识；下游再 SFT/微调。",
                ),
            ),
            (
                "backpropagation",
                (
                    "backpropagation",
                    "反向传播：从输出误差往回算梯度并更新权重。作用：神经网络训练的核心算法。",
                ),
            ),
            (
                "softmax",
                (
                    "softmax",
                    "把 logits 转为概率分布（和为 1）。用途：分类输出、注意力权重归一化。",
                ),
            ),
            (
                "cosmos",
                (
                    "Cosmos DB",
                    "Azure 全球分布式多模型数据库。用途：低延迟读写、多区域部署。适合 IoT、推荐、会话存储等。",
                ),
            ),
            (
                "cosmosdb",
                (
                    "Cosmos DB",
                    "Azure 全球分布式多模型数据库。用途：低延迟读写、多区域部署。适合 IoT、推荐、会话存储等。",
                ),
            ),
            (
                "kubernetes",
                (
                    "Kubernetes",
                    "容器编排平台。用途：自动部署、扩缩容、滚动更新。例：K8s 部署 LLM 推理服务多副本负载均衡。",
                ),
            ),
            (
                "docker",
                (
                    "Docker",
                    "容器化：把应用与依赖打包为镜像。用途：环境一致、快速部署。例：docker run 启动推理 API 容器。",
                ),
            ),
            (
                "grpc",
                (
                    "gRPC",
                    "高性能 RPC 框架，常用 Protobuf 序列化。用途：服务间低延迟通信。例：Themis 托盘与 service 用 gRPC 推字幕。",
                ),
            ),
            (
                "pytorch",
                (
                    "PyTorch",
                    "主流深度学习框架，动态图易调试。用途：LLM 训练/推理、研究原型。例：Hugging Face 多数模型支持 PyTorch。",
                ),
            ),
            (
                "tensorflow",
                (
                    "TensorFlow",
                    "Google 深度学习框架。用途：训练部署、TPU 加速、TF Serving 线上推理。",
                ),
            ),
            (
                "langchain",
                (
                    "LangChain",
                    "LLM 应用开发框架。用途：快速搭 RAG、Agent、工具链。用法：链式组合 retriever + LLM + memory。",
                ),
            ),
            (
                "llamaindex",
                (
                    "LlamaIndex",
                    "数据连接型 LLM 框架。用途：文档索引、检索、RAG 流水线。例：导入 PDF 建索引后问答。",
                ),
            ),
            (
                "chromadb",
                (
                    "ChromaDB",
                    "开源向量数据库。用途：存 embedding、语义检索。用法：本地 RAG 原型常用 Chroma 做向量存储。",
                ),
            ),
            (
                "weaviate",
                (
                    "Weaviate",
                    "开源向量搜索引擎。用途：混合检索（向量+关键词）。支持 GraphQL 查询与多租户。",
                ),
            ),
            (
                "pinecone",
                (
                    "Pinecone",
                    "托管向量数据库服务。用途：大规模 embedding 检索，免运维。例： SaaS 产品存百万文档向量。",
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
