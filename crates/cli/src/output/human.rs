use prism_core::types::report::DiagnosticReport;

use crate::output::renderers::{
    render_cause_list, render_error_card, render_fix_list, render_section_header,
    render_fee_breakdown, render_resource_summary,
};

pub fn print_report(report: &DiagnosticReport) -> anyhow::Result<()> {
    println!("{}", render_error_card(report));
    println!();

    println!("{}", render_section_header("Transaction Summary"));
    println!(
        "Error: {} ({}:{})",
        report.error_name, report.error_category, report.error_code
    );
    println!("Summary: {}", report.summary);

    if let Some(context) = &report.transaction_context {
        println!();
        print!("{}", render_resource_summary(&context.resources));
        println!();
        print!("{}", render_fee_breakdown(&context.fee));
    }

    if !report.root_causes.is_empty() {
        println!();
        println!("{}", render_cause_list(&report.root_causes));
    }

    if !report.suggested_fixes.is_empty() {
        println!();
        println!("{}", render_fix_list(&report.suggested_fixes));
    }

    if let Some(attribution) = &report.cross_contract_attribution {
        println!();
        println!("{}", render_section_header("Cross-Contract Failure Attribution"));
        println!("Origin Contract : {}", attribution.contract_address);
        if let Some(fn_name) = &attribution.function_name {
            println!("Failed Function : {fn_name}");
        }
        println!("Call Depth      : {}", attribution.call_depth);
        println!("Details         : {}", attribution.origin_description);
    }

    Ok(())
}
