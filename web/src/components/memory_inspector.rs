use leptos::prelude::*;

use ux::u18;

use reactive_stores::Field;

#[component]
pub fn MemoryInspector(
    #[prop(into)] memory: Field<Vec<u18>>,
    #[prop(into)] p: Field<u18>,
) -> impl IntoView {
    view! {
        <div class = "pane" style = "display: flex;">
            <div class = "memory-container">
            <div class = "memory">
                <table>
                    <thead>
                    <tr>
                        <th></th>
                        {
                            (0..8).into_iter().map(|idx| view! {<th> { format!("{:6o}", idx) } </th>}).collect_view()
                        }
                    </tr>
                    </thead>
                {
                    move ||
                    {
                        let memory = memory.get();
                        let p = p.get();

                        memory.chunks(8)
                              .into_iter()
                              .enumerate()
                              .map(|(base_addr, values)|
                                {
                                    let line = values.iter().enumerate()
                                                     .map(|(idx, value)| {
                                                        let addr = u18::new((base_addr * 8 + idx) as u32);
                                                        view!{ <td class = {if addr == p {"memory_p_register"} else {""} }> { format!("{:06o}", value) } </td> }
                                                    })
                                                     .collect_view();

                                    view!{<tr><th scope="row">{format!("{:06o}", base_addr * 8)}</th> { line } </tr>}
                                }
                              ).collect_view()
                    }
                }
                </table>
            </div>
            </div>
        </div>
    }
}
