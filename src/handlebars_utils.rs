handlebars::handlebars_helper!(is_some_string: |option: Option<String>| option.is_some() );

use handlebars::*;
use serde_json::Value;
use tracing::debug;

pub(crate) fn prepare<'reg>() -> Handlebars<'reg> {
    let mut handlebars = Handlebars::new();
    handlebars.set_dev_mode(true);
    handlebars.register_helper("is-some-string", Box::new(is_some_string));
    handlebars.register_decorator("switch", Box::new(switch));
    handlebars.register_helper("case", Box::new(case));
    handlebars.register_helper("format_file_size", Box::new(format_file_size));
    handlebars
        .register_templates_directory(
            "./templates",
            DirectorySourceOptions {
                tpl_extension: ".hbs".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
    handlebars
}

fn switch<'reg: 'rc, 'rc>(
    d: &Decorator,
    _: &Handlebars,
    ctx: &Context,
    rc: &mut RenderContext,
) -> Result<(), RenderError> {
    let switch_param = d.param(0).ok_or(RenderErrorReason::ParamNotFoundForName(
        "switch",
        "switch param not found".to_string(),
    ))?;
    // modify json object
    let mut new_ctx = ctx.clone();
    {
        let new_value = switch_param.value().clone();
        println!("new_value: {:?}", new_value);
        let data = new_ctx.data_mut();
        if let Some(ref mut m) = data.as_object_mut() {
            m.insert("my-drive-switch".to_string(), new_value);
        }
    }
    rc.set_context(new_ctx);
    Ok(())
}

fn case<'reg, 'rc>(
    h: &Helper<'rc>,
    r: &'reg Handlebars<'reg>,
    ctx: &'rc Context,
    rc: &mut RenderContext<'reg, 'rc>,
    out: &mut dyn Output,
) -> HelperResult {
    let actual = ctx.data();
    let expected = h.param(0).unwrap().value();
    debug!("case: {:?} == {:?}", actual, expected);
    if expected == actual {
        h.template()
            .map(|t| {
                let v = h.param(0).unwrap().value();
                rc.set_context(Context::from(v.clone()));
                t.render(r, ctx, rc, out)
            })
            .unwrap_or(Ok(()))
    } else {
        Ok(())
    }
}

handlebars_helper!(format_file_size: |v: Value|  {
    if let Some(mut bytes) = v.as_u64() {
        let mut i: usize = 0;
        let postfixes = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

        while bytes >= 1024 && i < postfixes.len() - 1 {
            bytes /= 1024;
            i += 1;
        }

        let rendered = format!("{} {}", bytes, postfixes[i]);
        return Ok(ScopedJson::Derived(Value::String(rendered)));
    }
    String::new()
});

#[cfg(test)]
mod test {
    use handlebars::Handlebars;

    #[test]
    fn test_handlebars_case() {
        let mut handlebars = Handlebars::new();
        handlebars.register_decorator("switch", Box::new(super::switch));
        handlebars.register_helper("case", Box::new(super::case));
        let template = "{{#*switch test}}>{{my-drive-switch}}<{{#if (eq my-drive-switch 1)}}one{{/if}}{{#if (eq my-drive-switch 2)}}2{{/if}}{{#case 2}}two{{/case}}{{#case 3}}three{{/case}}{{/switch}}";
        assert_eq!(
            handlebars
                .render_template(template, &serde_json::json!({"test":1}))
                .unwrap(),
            "one".to_owned()
        );
        assert_eq!(
            handlebars
                .render_template(template, &serde_json::json!({"test":2}))
                .unwrap(),
            "two".to_owned()
        );
    }
}
