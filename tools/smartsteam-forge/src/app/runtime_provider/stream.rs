use std::sync::mpsc;

use smartsteam_forge::{
    FinalPayloadSummary, ForgeProvider, ForgeSession, SessionAnswer, StreamEvent,
};

use super::super::provider::ProviderEvent;

pub(super) fn stream_prompt(
    provider: &ForgeProvider,
    session: &mut ForgeSession,
    prompt: &str,
    tx: &mpsc::Sender<ProviderEvent>,
    saw_text: &mut bool,
    saw_done: &mut bool,
) -> Result<SessionAnswer, String> {
    session.stream_prompt(provider, prompt, &mut |event| {
        match event {
            StreamEvent::Delta(text) => {
                *saw_text = true;
                tx.send(ProviderEvent::Delta(text))
                    .map_err(|error| format!("send delta to UI failed: {error}"))?;
            }
            StreamEvent::Stage(stage) => {
                tx.send(ProviderEvent::Stage(stage))
                    .map_err(|error| format!("send stage to UI failed: {error}"))?;
            }
            StreamEvent::Meta(meta) => {
                tx.send(ProviderEvent::Meta(meta))
                    .map_err(|error| format!("send meta to UI failed: {error}"))?;
            }
            StreamEvent::Final(payload) => {
                let summary = FinalPayloadSummary::parse(&payload);
                tx.send(ProviderEvent::Status(summary.status_line()))
                    .map_err(|error| format!("send final status to UI failed: {error}"))?;
                if let Some(report) = summary.gate_report() {
                    tx.send(ProviderEvent::GateReport(report))
                        .map_err(|error| format!("send gate report to UI failed: {error}"))?;
                }
            }
            StreamEvent::Done => {
                *saw_done = true;
                tx.send(ProviderEvent::Done)
                    .map_err(|error| format!("send done to UI failed: {error}"))?;
            }
            StreamEvent::Error(error) => {
                *saw_done = true;
                return Err(format!("backend stream error event: {error}"));
            }
            StreamEvent::Heartbeat(heartbeat) => {
                tx.send(ProviderEvent::Heartbeat(heartbeat))
                    .map_err(|error| format!("send heartbeat to UI failed: {error}"))?;
            }
            StreamEvent::Status(status) => {
                tx.send(ProviderEvent::Status(status))
                    .map_err(|error| format!("send status to UI failed: {error}"))?;
            }
            StreamEvent::Message { event, data } => {
                tx.send(ProviderEvent::Status(format!("{event}: {data}")))
                    .map_err(|error| format!("send stream message to UI failed: {error}"))?;
            }
        }
        Ok(())
    })
}

pub(super) fn final_answer_differs(answer: &SessionAnswer) -> bool {
    let final_answer = answer.assistant_message.trim();
    !final_answer.is_empty() && final_answer != answer.streamed_text.trim()
}
