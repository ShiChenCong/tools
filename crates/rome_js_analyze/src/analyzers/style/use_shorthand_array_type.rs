use rome_analyze::{context::RuleContext, declare_rule, ActionCategory, Ast, Rule, RuleDiagnostic};
use rome_console::markup;
use rome_diagnostics::Applicability;
use rome_js_factory::make;
use rome_js_syntax::{AnyTsType, TriviaPieceKind, TsReferenceType, TsTypeArguments, T};
use rome_rowan::{AstNode, AstSeparatedList, BatchMutationExt};

use crate::JsRuleAction;

declare_rule! {
    /// When expressing array types, this rule promotes the usage of `T[]` shorthand instead of `Array<T>`.
    ///
    /// ## Examples
    ///
    /// ### Invalid
    /// ```ts,expect_diagnostic
    /// let valid: Array<foo>;
    /// ```
    ///
    /// ```ts,expect_diagnostic
    /// let invalid2: Promise<Array<string>>;
    /// ```
    ///
    /// ```ts,expect_diagnostic
    /// let invalid3: Array<Foo<Bar>>;
    /// ```
    ///
    /// ```ts,expect_diagnostic
    /// let invalid: Array<[number, number]>;
    /// ```
    ///
    /// ```ts,expect_diagnostic
    /// let invalid: Array<[number, number]>;
    /// ```
    ///
    /// ### Valid
    ///
    /// ```ts
    /// let valid: Array<Foo | Bar>;
    /// let valid: Array<keyof Bar>;
    /// let valid: Array<foo | bar>;
    /// ```
    pub(crate) UseShorthandArrayType  {
        version: "0.7.0",
        name: "useShorthandArrayType",
        recommended: false,
    }
}

impl Rule for UseShorthandArrayType {
    type Query = Ast<TsReferenceType>;
    type State = AnyTsType;
    type Signals = Option<Self::State>;
    type Options = ();

    fn run(ctx: &RuleContext<Self>) -> Option<Self::State> {
        let node = ctx.query();
        let type_arguments = node.type_arguments()?;
        is_array_reference(node).and_then(|ret| {
            if ret {
                convert_to_array_type(type_arguments)
            } else {
                None
            }
        })
    }

    fn diagnostic(ctx: &RuleContext<Self>, _: &Self::State) -> Option<RuleDiagnostic> {
        let node = ctx.query();

        Some(RuleDiagnostic::new(
            rule_category!(),
            node.range(),
            markup! {

                "Use "<Emphasis>"shorthand T[] syntax"</Emphasis>" instead of "<Emphasis>"Array<T> syntax."</Emphasis>
            },
        ))
    }

    fn action(ctx: &RuleContext<Self>, state: &Self::State) -> Option<JsRuleAction> {
        let node = ctx.query();
        let mut mutation = ctx.root().begin();

        mutation.replace_node(AnyTsType::TsReferenceType(node.clone()), state.clone());

        Some(JsRuleAction {
            category: ActionCategory::QuickFix,
            applicability: Applicability::MaybeIncorrect,
            message: markup! { "Use "<Emphasis>"shorthand T[] syntax"</Emphasis>" to replace" }
                .to_owned(),
            mutation,
        })
    }
}

fn is_array_reference(ty: &TsReferenceType) -> Option<bool> {
    let name = ty.name().ok()?;
    name.as_js_reference_identifier().and_then(|identifier| {
        let name = identifier.value_token().ok()?;
        Some(name.text_trimmed() == "Array")
    })
}

fn convert_to_array_type(type_arguments: TsTypeArguments) -> Option<AnyTsType> {
    if type_arguments.ts_type_argument_list().len() > 0 {
        let types_array = type_arguments
            .ts_type_argument_list()
            .into_iter()
            .filter_map(|param| {
                let param = param.ok()?;
                let element_type = match &param {
                    // Intersection or higher types
                    AnyTsType::TsUnionType(_)
                    | AnyTsType::TsIntersectionType(_)
                    | AnyTsType::TsFunctionType(_)
                    | AnyTsType::TsConstructorType(_)
                    | AnyTsType::TsConditionalType(_)
                    | AnyTsType::TsTypeOperatorType(_)
                    | AnyTsType::TsInferType(_)
                    | AnyTsType::TsMappedType(_) => None,

                    AnyTsType::TsReferenceType(ty) if is_array_reference(ty).unwrap_or(false) => {
                        if let Some(type_arguments) = ty.type_arguments() {
                            convert_to_array_type(type_arguments)
                        } else {
                            Some(param)
                        }
                    }
                    _ => Some(param),
                };
                element_type.map(|element_type| {
                    AnyTsType::TsArrayType(make::ts_array_type(
                        element_type,
                        make::token(T!['[']),
                        make::token(T![']']),
                    ))
                })
            })
            .collect::<Vec<_>>();
        match types_array.len() {
            0 => {}
            1 => {
                // SAFETY: We know that `length` of `array_types` is 1, so unwrap the first element should be safe.
                let first_type = types_array.into_iter().next().unwrap();
                return Some(first_type);
            }
            length => {
                let ts_union_type_builder = make::ts_union_type(make::ts_union_type_variant_list(
                    types_array.into_iter(),
                    (0..length - 1).map(|_| {
                        make::token(T![|])
                            .with_leading_trivia(std::iter::once((
                                TriviaPieceKind::Whitespace,
                                " ",
                            )))
                            .with_trailing_trivia(std::iter::once((
                                TriviaPieceKind::Whitespace,
                                " ",
                            )))
                    }),
                ));
                return Some(AnyTsType::TsUnionType(ts_union_type_builder.build()));
            }
        }
    }
    None
}
