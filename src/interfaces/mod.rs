use serde::Serialize;
use serde_with::skip_serializing_none;

pub mod command_line;
pub mod web;
pub mod server;

/// 向用户反馈的消息类型
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[skip_serializing_none]
pub enum 消息 {
    TrialMax {
        temperature: f64,
        accept_rate: f64,
    },
    TrialMin {
        temperature: f64,
        improve_rate: f64,
    },
    Parameters {
        t_max: f64,
        t_min: f64,
    },
    Progress {
        steps: usize,
        temperature: f64,
        metric: String,
    },
    BetterSolution {
        metric: String,
        config: String,
        save: bool,
    },
    Elapsed {
        time: u64,
    },
}

/// 定义了向用户报告消息的接口，用于统一命令行和图形界面的输出方式
///
/// 命令行界面、图形界面只需要各自实现 post 方法，就可向用户报告各种用户数据
pub trait 界面 {
    fn 发送(&self, 消息: 消息);
}
