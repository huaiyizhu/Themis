//! Rule-based extraction: acronyms, glossary hits, question detection.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use themis_core::{AnalysisResult, QuestionInsight, TermInsight};

static ACRONYM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z]{2,8}\b").expect("acronym regex"));
static QUESTION_EN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]\s*([^.!?]*\?)").expect("question en"));
static QUESTION_ZH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^。！？]*[？?]").expect("question zh"));

static GLOSSARY: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("NBA", "National Basketball Association，美国职业篮球联赛"),
        ("AI", "Artificial Intelligence，人工智能"),
        ("ML", "Machine Learning，机器学习"),
        ("LLM", "Large Language Model，大语言模型"),
        ("RAG", "Retrieval-Augmented Generation，检索增强生成"),
        ("GPU", "Graphics Processing Unit，图形处理器，常用于深度学习加速"),
        ("CPU", "Central Processing Unit，中央处理器"),
        ("API", "Application Programming Interface，应用程序接口"),
        ("HTTP", "Hypertext Transfer Protocol，超文本传输协议"),
        ("REST", "Representational State Transfer，一种 Web API 架构风格"),
        ("SQL", "Structured Query Language，结构化查询语言"),
        ("JSON", "JavaScript Object Notation，常用数据交换格式"),
        ("OCR", "Optical Character Recognition，光学字符识别"),
        ("STT", "Speech-to-Text，语音转文字"),
        ("TTS", "Text-to-Speech，文字转语音"),
        ("AWS", "Amazon Web Services，亚马逊云计算平台"),
        ("Azure", "Microsoft Azure，微软云计算平台"),
        ("GPT", "Generative Pre-trained Transformer，生成式预训练 Transformer 模型"),
        ("UI", "User Interface，用户界面"),
        ("UX", "User Experience，用户体验"),
        ("IoT", "Internet of Things，物联网"),
        ("VPN", "Virtual Private Network，虚拟专用网络"),
        ("DNS", "Domain Name System，域名系统"),
        ("CDN", "Content Delivery Network，内容分发网络"),
        ("KPI", "Key Performance Indicator，关键绩效指标"),
        ("ROI", "Return on Investment，投资回报率"),
        ("CEO", "Chief Executive Officer，首席执行官"),
        ("CTO", "Chief Technology Officer，首席技术官"),
    ])
});

pub fn analyze_heuristic(transcript: &str) -> AnalysisResult {
    let text = transcript.trim();
    if text.is_empty() {
        return AnalysisResult::default();
    }

    let mut result = AnalysisResult::default();

    for cap in ACRONYM_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let term = m.as_str().to_string();
            if term.len() < 2 {
                continue;
            }
            if !result.keywords.iter().any(|k| k == &term) {
                result.keywords.push(term.clone());
            }
            if let Some(exp) = GLOSSARY.get(term.as_str()) {
                result.terms.push(TermInsight {
                    term: term.clone(),
                    explanation: (*exp).to_string(),
                });
            }
        }
    }

    // Chinese / mixed technical terms (simple token scan)
    for (term, exp) in GLOSSARY.iter() {
        if text.contains(term) && !result.terms.iter().any(|t| t.term == *term) {
            if term.chars().any(|c| c.is_ascii_uppercase()) {
                continue; // already handled by acronym pass
            }
            result.terms.push(TermInsight {
                term: (*term).to_string(),
                explanation: (*exp).to_string(),
            });
        }
    }

  // Longer English words that look like domain terms (TitleCase or technical)
    let word_re = Regex::new(r"\b[A-Za-z]{4,}\b").unwrap();
    for cap in word_re.captures_iter(text) {
        if let Some(w) = cap.get(0) {
            let word = w.as_str();
            if word.chars().all(|c| c.is_ascii_uppercase()) {
                continue;
            }
            if word.chars().next().is_some_and(|c| c.is_uppercase())
                && !result.keywords.iter().any(|k| k.eq_ignore_ascii_case(word))
            {
                result.keywords.push(word.to_string());
            }
        }
    }
    result.keywords.truncate(8);

    for q in extract_questions(text) {
        result.questions.push(QuestionInsight {
            question: q.clone(),
            answer: brief_question_hint(&q),
        });
    }

    result
}

fn extract_questions(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for cap in QUESTION_ZH_RE.find_iter(text) {
        let q = cap.as_str().trim().to_string();
        if q.len() >= 4 && !out.contains(&q) {
            out.push(q);
        }
    }
    for cap in QUESTION_EN_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let q = m.as_str().trim().to_string();
            if q.len() >= 6 && !out.contains(&q) {
                out.push(q);
            }
        }
    }
    if text.contains('?') || text.contains('？') {
        let whole = text.trim().to_string();
        if (whole.ends_with('?') || whole.ends_with('？'))
            && whole.len() >= 6
            && !out.contains(&whole)
        {
            out.push(whole);
        }
    }
    out.truncate(3);
    out
}

fn brief_question_hint(question: &str) -> String {
    if question.contains("RAG") || question.to_lowercase().contains("rag") {
        return "RAG 通过检索外部知识再生成回答，常用于减少幻觉、接入私有文档。".into();
    }
    if question.contains("什么") || question.contains("what") {
        return "（初步）这是定义/概念类问题，可结合上下文中的术语进一步查证。".into();
    }
    if question.contains("为什么") || question.contains("why") {
        return "（初步）这是因果/动机类问题，需结合前后文与领域背景分析。".into();
    }
    if question.contains("如何") || question.contains("怎么") || question.to_lowercase().contains("how") {
        return "（初步）这是方法/步骤类问题，可拆解为流程或关键条件来回答。".into();
    }
    "（初步）已识别为问题句，启用 LLM 配置后可获得更完整回答。".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rag_and_nba() {
        let r = analyze_heuristic("What is RAG in AI? NBA games tonight.");
        assert!(r.terms.iter().any(|t| t.term == "RAG"));
        assert!(r.terms.iter().any(|t| t.term == "NBA"));
    }
}
