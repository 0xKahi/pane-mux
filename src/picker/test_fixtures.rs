use crate::{
    mux::{BackendKind, Pane, Session, Tree, Window},
    tmux::FakeTmux,
};

pub(super) fn rex_shaped() -> FakeTmux {
    let pane = |id: &str, index, command: &str, floating| Pane {
        id: format!("block:{id}"),
        index,
        command: command.into(),
        active: index == 1,
        floating,
        ..Pane::default()
    };
    let window = |name: &str, index, panes| Window {
        id: format!("window:{name}"),
        name: name.into(),
        index,
        active: index == 1,
        panes,
        ..Window::default()
    };
    FakeTmux::new(Tree {
        sessions: vec![
            Session {
                id: "session:kahi".into(),
                name: "kahi".into(),
                windows: vec![
                    window(
                        "config",
                        1,
                        vec![
                            pane("nvim", 1, "nvim", false),
                            pane("float", 2, "zsh", true),
                        ],
                    ),
                    window("pi", 2, vec![pane("pi", 1, "zsh", false)]),
                ],
                ..Session::default()
            },
            Session {
                id: "session:shuffle".into(),
                name: "shuffle".into(),
                windows: vec![window("shell", 1, vec![pane("shell", 1, "zsh", false)])],
                ..Session::default()
            },
        ],
    })
    .with_kind(BackendKind::Rex)
    .with_cross_session_moves(false)
}
