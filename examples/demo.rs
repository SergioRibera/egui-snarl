use std::{cell::RefCell, collections::HashMap};

use eframe::App;
use egui::{pos2, Color32, InnerResponse, Ui};
use egui_snarl::{
    ui::{Effects, Forbidden, InPin, OutPin, PinInfo, SnarlStyle, SnarlViewer},
    InPinId, Snarl,
};

#[derive(Clone)]
enum DemoNode {
    /// Node with single input.
    /// Displays the value of the input.
    Sink,

    /// Value node with a single output.
    /// The value is editable in UI.
    Integer(i32),

    /// Value node with a single output.
    String(String),

    /// Converts URI to Image
    Show(String),

    /// Expression node with a single output.
    /// It has number of inputs equal to number of variables in the expression.
    ExprNode(ExprNode),
}

struct DemoViewer;

impl SnarlViewer<DemoNode> for DemoViewer {
    fn node_picker(&mut self, _ui: &mut Ui) -> egui::InnerResponse<Option<DemoNode>> {
        todo!()
    }

    #[inline]
    fn connect(
        &mut self,
        from: &OutPin<DemoNode>,
        to: &InPin<DemoNode>,
        effects: &mut Effects<DemoNode>,
    ) -> Result<(), Forbidden> {
        // Validate connection
        match (&*from.node.borrow(), &*to.node.borrow()) {
            (DemoNode::Sink, _) => {
                unreachable!("Sink node has no outputs")
            }
            (_, DemoNode::Integer(_)) => {
                unreachable!("Integer node has no inputs")
            }
            (_, DemoNode::String(_)) => {
                unreachable!("String node has no inputs")
            }
            (DemoNode::Integer(_), DemoNode::Show(_)) => {
                return Err(Forbidden);
            }
            (DemoNode::Show(_), DemoNode::Show(_)) => {
                return Err(Forbidden);
            }
            (_, DemoNode::Sink) => {}
            (DemoNode::String(_), DemoNode::Show(_)) => {}
            (DemoNode::ExprNode(_), DemoNode::ExprNode(_)) => {}
            (DemoNode::Integer(_), DemoNode::ExprNode(_)) => {}
            (DemoNode::String(_), DemoNode::ExprNode(_)) => {
                return Err(Forbidden);
            }
            (DemoNode::Show(_), DemoNode::ExprNode(_)) => {
                return Err(Forbidden);
            }
            (DemoNode::ExprNode(_), DemoNode::Show(_)) => {
                return Err(Forbidden);
            }
        }

        for remote in &to.remotes {
            effects.disconnect(remote.id, to.id);
        }

        effects.connect(from.id, to.id);
        Ok(())
    }

    fn size_hint(&self, _node: &DemoNode) -> egui::Vec2 {
        egui::vec2(130.0, 50.0)
    }

    fn title(&mut self, node: &DemoNode) -> &str {
        match node {
            DemoNode::Sink => "Sink",
            DemoNode::Integer(_) => "Integer",
            DemoNode::String(_) => "String",
            DemoNode::Show(_) => "Show",
            DemoNode::ExprNode(_) => "Expr",
        }
    }

    fn show_content(
        &mut self,
        node_idx: usize,
        node: &RefCell<DemoNode>,
        inputs: &[InPin<DemoNode>],
        _outputs: &[OutPin<DemoNode>],
        ui: &mut Ui,
        effects: &mut Effects<DemoNode>,
    ) -> egui::Response {
        match &mut *node.borrow_mut() {
            DemoNode::ExprNode(expr_node) => {
                let r = ui.text_edit_singleline(&mut expr_node.text);

                match syn::parse_str(&expr_node.text) {
                    Ok(expr) => {
                        expr_node.expr = expr;

                        let values = Iterator::zip(
                            expr_node.bindings.iter().map(|s| &**s),
                            expr_node.values.iter().copied(),
                        )
                        .collect::<HashMap<&str, f32>>();

                        let mut new_bindings = Vec::new();
                        expr_node.expr.extend_bindings(&mut new_bindings);

                        for (idx, name) in expr_node.bindings.iter().enumerate() {
                            let new_idx =
                                new_bindings.iter().position(|new_name| *new_name == *name);

                            match new_idx {
                                None => {
                                    effects.drop_inputs(inputs[idx].id);
                                }
                                Some(new_idx) if new_idx != idx => {
                                    let new_in_pin = InPinId {
                                        node: node_idx,
                                        input: new_idx,
                                    };
                                    for remote in &inputs[idx].remotes {
                                        effects.disconnect(remote.id, inputs[idx].id);
                                        effects.connect(remote.id, new_in_pin);
                                    }
                                }
                                _ => {}
                            }
                        }

                        let new_values = new_bindings
                            .iter()
                            .map(|name| values.get(&**name).copied().unwrap_or(0.0))
                            .collect::<Vec<_>>();

                        expr_node.bindings = new_bindings;
                        expr_node.values = new_values;
                    }
                    Err(_) => {}
                }

                r
            }
            _ => ui.interact(egui::Rect::ZERO, egui::Id::NULL, egui::Sense::hover()),
        }
    }

    fn inputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 1,
            DemoNode::Integer(_) => 0,
            DemoNode::String(_) => 0,
            // DemoNode::Add(values) => values.len() + 1,
            DemoNode::Show(_) => 1,
            DemoNode::ExprNode(expr_node) => expr_node.bindings.len(),
        }
    }

    fn outputs(&mut self, node: &DemoNode) -> usize {
        match node {
            DemoNode::Sink => 0,
            DemoNode::Integer(_) => 1,
            DemoNode::String(_) => 1,
            // DemoNode::Add(_) => 1,
            DemoNode::Show(_) => 1,
            DemoNode::ExprNode(_) => 1,
        }
    }

    fn show_input(
        &mut self,
        pin: &InPin<DemoNode>,
        ui: &mut Ui,
        _effects: &mut Effects<DemoNode>,
    ) -> egui::InnerResponse<PinInfo> {
        let demo_node = pin.node.borrow().clone();
        match demo_node {
            DemoNode::Sink => {
                assert_eq!(pin.id.input, 0, "Sink node has only one input");

                match &*pin.remotes {
                    [] => {
                        let r = ui.label("None");
                        InnerResponse::new(PinInfo::circle().with_fill(Color32::GRAY), r)
                    }
                    [remote] => match *remote.node.borrow() {
                        DemoNode::Sink => unreachable!("Sink node has no outputs"),
                        DemoNode::Integer(value) => {
                            assert_eq!(remote.id.output, 0, "Integer node has only one output");
                            let r = ui.label(format!("{}", value));
                            InnerResponse::new(PinInfo::square().with_fill(Color32::RED), r)
                        }
                        DemoNode::String(ref value) => {
                            assert_eq!(remote.id.output, 0, "String node has only one output");
                            let r = ui.label(format!("{:?}", value));
                            InnerResponse::new(PinInfo::triangle().with_fill(Color32::GREEN), r)
                        }
                        DemoNode::ExprNode(ref expr) => {
                            assert_eq!(remote.id.output, 0, "Expr node has only one output");
                            let r = ui.label(format!("{}", expr.eval()));
                            InnerResponse::new(PinInfo::square().with_fill(Color32::RED), r)
                        }
                        DemoNode::Show(ref uri) => {
                            assert_eq!(remote.id.output, 0, "Show node has only one output");

                            let image = egui::Image::new(uri)
                                .fit_to_original_size(1.0)
                                .show_loading_spinner(true);
                            let r = ui.add(image);

                            InnerResponse::new(PinInfo::circle().with_fill(Color32::GOLD), r)
                        }
                    },
                    _ => unreachable!("Sink input has only one wire"),
                }
            }
            DemoNode::Integer(_) => {
                unreachable!("Integer node has no inputs")
            }
            DemoNode::String(_) => {
                unreachable!("String node has no inputs")
            }
            DemoNode::Show(_) => match &*pin.remotes {
                [] => match &mut *pin.node.borrow_mut() {
                    DemoNode::Show(uri) => {
                        let r = ui.text_edit_singleline(uri);
                        InnerResponse::new(PinInfo::triangle().with_fill(Color32::GREEN), r)
                    }
                    _ => unreachable!(),
                },
                [remote] => match remote.node.borrow().clone() {
                    DemoNode::Sink => unreachable!("Sink node has no outputs"),
                    DemoNode::Show(_) => {
                        unreachable!("Show node has no outputs")
                    }
                    DemoNode::Integer(_) | DemoNode::ExprNode(_) => {
                        unreachable!("Invalid connection")
                    }
                    DemoNode::String(value) => match &mut *pin.node.borrow_mut() {
                        DemoNode::Show(uri) => {
                            *uri = value.clone();
                            let r = ui.text_edit_singleline(&mut &**uri);
                            InnerResponse::new(PinInfo::triangle().with_fill(Color32::GREEN), r)
                        }
                        _ => unreachable!(),
                    },
                },
                _ => unreachable!("Sink input has only one wire"),
            },
            DemoNode::ExprNode(expr_node) => {
                if pin.id.input < expr_node.bindings.len() {
                    match &*pin.remotes {
                        [] => match &mut *pin.node.borrow_mut() {
                            DemoNode::ExprNode(expr_node) => ui.horizontal(|ui| {
                                ui.label(&expr_node.bindings[pin.id.input]);
                                ui.add(egui::DragValue::new(&mut expr_node.values[pin.id.input]));
                                PinInfo::square().with_fill(Color32::RED)
                            }),
                            _ => unreachable!(),
                        },
                        [remote] => ui.horizontal(|ui| {
                            ui.label(&expr_node.bindings[pin.id.input]);

                            let remote_node = remote.node.borrow().clone();
                            match remote_node {
                                DemoNode::Sink => unreachable!("Sink node has no outputs"),
                                DemoNode::Integer(value) => {
                                    assert_eq!(
                                        remote.id.output, 0,
                                        "Integer node has only one output"
                                    );
                                    match &mut *pin.node.borrow_mut() {
                                        DemoNode::ExprNode(expr_node) => {
                                            expr_node.values[pin.id.input] = value as f32;
                                        }
                                        _ => unreachable!(),
                                    }
                                    ui.label(format!("{}", value));
                                    PinInfo::square().with_fill(Color32::RED)
                                }
                                DemoNode::ExprNode(expr_node) => {
                                    let value = expr_node.eval();

                                    assert_eq!(
                                        remote.id.output, 0,
                                        "Expr node has only one output"
                                    );
                                    match &mut *pin.node.borrow_mut() {
                                        DemoNode::ExprNode(expr_node) => {
                                            expr_node.values[pin.id.input] = value;
                                        }
                                        _ => unreachable!(),
                                    }
                                    ui.label(format!("{:0.2}", value));
                                    PinInfo::square().with_fill(Color32::RED)
                                }
                                DemoNode::Show(_) => {
                                    unreachable!("Show node has no outputs")
                                }
                                DemoNode::String(_) => {
                                    unreachable!("Invalid connection")
                                }
                            }
                        }),
                        _ => unreachable!("Expr pins has only one wire"),
                    }
                } else {
                    let r = ui.label("Removed");
                    egui::InnerResponse::new(PinInfo::circle().with_fill(Color32::BLACK), r)
                }
            }
        }
    }

    fn show_output(
        &mut self,
        pin: &OutPin<DemoNode>,
        ui: &mut Ui,
        _effects: &mut Effects<DemoNode>,
    ) -> egui::InnerResponse<PinInfo> {
        match *pin.node.borrow_mut() {
            DemoNode::Sink => {
                unreachable!("Sink node has no outputs")
            }
            DemoNode::Integer(ref mut value) => {
                assert_eq!(pin.id.output, 0, "Integer node has only one output");
                let r = ui.add(egui::DragValue::new(value));
                InnerResponse::new(PinInfo::square().with_fill(Color32::RED), r)
            }
            DemoNode::String(ref mut value) => {
                assert_eq!(pin.id.output, 0, "String node has only one output");
                let r = ui.text_edit_singleline(value);
                InnerResponse::new(PinInfo::triangle().with_fill(Color32::GREEN), r)
            }
            DemoNode::ExprNode(ref expr_node) => {
                let value = expr_node.eval();
                assert_eq!(pin.id.output, 0, "Add node has only one output");
                let r = ui.label(format!("{:0.2}", value));
                InnerResponse::new(PinInfo::square().with_fill(Color32::RED), r)
            }
            DemoNode::Show(_) => {
                let (_, r) = ui.allocate_exact_size(egui::Vec2::ZERO, egui::Sense::hover());
                InnerResponse::new(PinInfo::circle().with_fill(Color32::GOLD), r)
            }
        }
    }
}

pub struct DemoApp {
    snarl: Snarl<DemoNode>,
}

impl DemoApp {
    pub fn new() -> Self {
        let mut snarl = Snarl::new();

        snarl.add_node(DemoNode::Integer(42), pos2(10.0, 20.0));

        snarl.add_node(DemoNode::ExprNode(ExprNode::new()), pos2(30.0, 80.0));

        snarl.add_node(DemoNode::ExprNode(ExprNode::new()), pos2(40.0, 100.0));

        // snarl.add_node(DemoNode::String("".to_owned()), pos2(20.0, 150.0));

        // snarl.add_node(DemoNode::Show("".to_owned()), pos2(120.0, 20.0));

        // snarl.add_node(DemoNode::Sink, pos2(190.0, 60.0));

        DemoApp { snarl }
    }
}

impl App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close)
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.snarl.show(
                &mut DemoViewer,
                &SnarlStyle {
                    upscale_wire: true,
                    downscale_wire: false,
                    ..Default::default()
                },
                egui::Id::new("snarl"),
                ui,
            );
        });
    }
}

#[derive(Clone)]
struct ExprNode {
    text: String,
    bindings: Vec<String>,
    values: Vec<f32>,
    expr: Expr,
}

impl ExprNode {
    fn new() -> Self {
        ExprNode {
            text: format!("0"),
            bindings: Vec::new(),
            values: Vec::new(),
            expr: Expr::Val(0.0),
        }
    }

    fn eval(&self) -> f32 {
        self.expr.eval(&self.bindings, &self.values)
    }
}

#[derive(Clone, Copy)]
enum UnOp {
    Pos,
    Neg,
}

#[derive(Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone)]
enum Expr {
    Var(String),
    Val(f32),
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
    },
    BinOp {
        lhs: Box<Expr>,
        op: BinOp,
        rhs: Box<Expr>,
    },
}

impl Expr {
    fn eval(&self, bindings: &[String], args: &[f32]) -> f32 {
        let binding_index =
            |name: &str| bindings.iter().position(|binding| binding == name).unwrap();

        match self {
            Expr::Var(ref name) => args[binding_index(name)],
            Expr::Val(value) => *value,
            Expr::UnOp { op, ref expr } => match op {
                UnOp::Pos => expr.eval(bindings, args),
                UnOp::Neg => -expr.eval(bindings, args),
            },
            Expr::BinOp {
                ref lhs,
                op,
                ref rhs,
            } => match op {
                BinOp::Add => lhs.eval(bindings, args) + rhs.eval(bindings, args),
                BinOp::Sub => lhs.eval(bindings, args) - rhs.eval(bindings, args),
                BinOp::Mul => lhs.eval(bindings, args) * rhs.eval(bindings, args),
                BinOp::Div => lhs.eval(bindings, args) / rhs.eval(bindings, args),
            },
        }
    }

    fn extend_bindings(&self, bindings: &mut Vec<String>) {
        match self {
            Expr::Var(name) => {
                if !bindings.contains(name) {
                    bindings.push(name.clone());
                }
            }
            Expr::Val(_) => {}
            Expr::UnOp { expr, .. } => {
                expr.extend_bindings(bindings);
            }
            Expr::BinOp { lhs, rhs, .. } => {
                lhs.extend_bindings(bindings);
                rhs.extend_bindings(bindings);
            }
        }
    }
}

impl syn::parse::Parse for UnOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            Ok(UnOp::Pos)
        } else if lookahead.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            Ok(UnOp::Neg)
        } else {
            Err(lookahead.error())
        }
    }
}

impl syn::parse::Parse for BinOp {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Token![+]) {
            input.parse::<syn::Token![+]>()?;
            Ok(BinOp::Add)
        } else if lookahead.peek(syn::Token![-]) {
            input.parse::<syn::Token![-]>()?;
            Ok(BinOp::Sub)
        } else if lookahead.peek(syn::Token![*]) {
            input.parse::<syn::Token![*]>()?;
            Ok(BinOp::Mul)
        } else if lookahead.peek(syn::Token![/]) {
            input.parse::<syn::Token![/]>()?;
            Ok(BinOp::Div)
        } else {
            Err(lookahead.error())
        }
    }
}

impl syn::parse::Parse for Expr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let lhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let expr = content.parse::<Expr>()?;
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitFloat) {
            let lit = input.parse::<syn::LitFloat>()?;
            let value = lit.base10_parse::<f32>()?;
            let expr = Expr::Val(value);
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f32>()?;
            let expr = Expr::Val(value);
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            let expr = Expr::Var(ident.to_string());
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else {
            let unop = input.parse::<UnOp>()?;

            return Self::parse_with_unop(unop, input);
        }

        let binop = input.parse::<BinOp>()?;

        Self::parse_binop(Box::new(lhs), binop, input)
    }
}

impl Expr {
    fn parse_with_unop(op: UnOp, input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let lhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let expr = Expr::UnOp {
                op,
                expr: Box::new(content.parse::<Expr>()?),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitFloat) {
            let lit = input.parse::<syn::LitFloat>()?;
            let value = lit.base10_parse::<f32>()?;
            let expr = Expr::UnOp {
                op,
                expr: Box::new(Expr::Val(value)),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f32>()?;
            let expr = Expr::UnOp {
                op,
                expr: Box::new(Expr::Val(value)),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            let expr = Expr::UnOp {
                op,
                expr: Box::new(Expr::Var(ident.to_string())),
            };
            if input.is_empty() {
                return Ok(expr);
            }
            lhs = expr;
        } else {
            return Err(lookahead.error());
        }

        let op = input.parse::<BinOp>()?;

        Self::parse_binop(Box::new(lhs), op, input)
    }

    fn parse_binop(lhs: Box<Expr>, op: BinOp, input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        let rhs;
        if lookahead.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            rhs = Box::new(content.parse::<Expr>()?);
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::LitFloat) {
            let lit = input.parse::<syn::LitFloat>()?;
            let value = lit.base10_parse::<f32>()?;
            rhs = Box::new(Expr::Val(value));
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::LitInt) {
            let lit = input.parse::<syn::LitInt>()?;
            let value = lit.base10_parse::<f32>()?;
            rhs = Box::new(Expr::Val(value));
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else if lookahead.peek(syn::Ident) {
            let ident = input.parse::<syn::Ident>()?;
            rhs = Box::new(Expr::Var(ident.to_string()));
            if input.is_empty() {
                return Ok(Expr::BinOp { lhs, op, rhs });
            }
        } else {
            return Err(lookahead.error());
        }

        let next_op = input.parse::<BinOp>()?;

        match (op, next_op) {
            (BinOp::Add | BinOp::Sub, BinOp::Mul | BinOp::Div) => {
                let rhs = Self::parse_binop(rhs, next_op, input)?;
                Ok(Expr::BinOp {
                    lhs,
                    op,
                    rhs: Box::new(rhs),
                })
            }
            _ => {
                let lhs = Expr::BinOp { lhs, op, rhs };
                Self::parse_binop(Box::new(lhs), next_op, input)
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "egui-snarl demo",
        native_options,
        Box::new(|_| Box::new(DemoApp::new())),
    )
}
