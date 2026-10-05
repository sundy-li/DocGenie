//! Native workbench DSL; separate source keeps --hot block matching stable.
use super::{App, Outline};
use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.draw.KeyCode
    let Caption = Label{draw_text +: {color: #x626b78 text_style +: {font_size: 12}}}
    let Action = ButtonFlatter{height: 32 padding: Inset{left: 10 right: 10 top: 6 bottom: 6} draw_text +: {color: #x303740 color_hover: #x2162c2 color_down: #x2162c2}}
    let IconAction = ButtonFlatterIcon{
        width: 32 height: 32
        padding: Inset{left: 0 right: 0 top: 0 bottom: 0}
        align: Align{x: 0.5 y: 0.5}
        grab_key_focus: false
        icon_walk: Walk{width: 18 height: 18}
        draw_icon +: {color: #x4b5563}
        draw_bg +: {border_radius: 4.0 border_size: 0.0 color: #00000000 color_hover: #xeef1f5 color_down: #xe2e7ee color_focus: #00000000}
    }
    mod.widgets.Outline = #(Outline::register_widget(vm))
    let Navigation = mod.widgets.Outline{
        width: Fill height: Fill
        list := PortalList{width: Fill height: Fill flow: Down
            Row := View{width: Fill height: Fit padding: Inset{left: 8 right: 8 top: 2 bottom: 2}
                heading := ButtonFlatter{width: Fill align: Align{x: 0.0 y: 0.5} text: "标题" draw_text +: {color: #x555f6d color_hover: #x7c3aed}}
            }
            Empty := View{width: Fill height: 1}
        }
    }
    let TreeFold = ButtonFlatterIcon{
        width: 20 height: 22
        padding: Inset{left: 0 right: 0 top: 0 bottom: 0}
        align: Align{x: 0.5 y: 0.5}
        grab_key_focus: false
        icon_walk: Walk{width: 12 height: 12}
        draw_icon +: {color: #x6b7280}
        draw_bg +: {border_radius: 3.0 border_size: 0.0 color: #00000000 color_hover: #x00000014 color_down: #x00000024 color_focus: #00000000}
    }
    let FileTree = mod.widgets.Outline{
        document_mode: true
        width: Fill height: Fill
        list := PortalList{width: Fill height: Fill flow: Down
            Row := RectView{width: Fill height: 34 flow: Right align: Align{x: 0.0 y: 0.5} padding: Inset{left: 8 right: 8} draw_bg +: {color: #x00000000 border_radius: 6.0 border_size: 0.0}
                indent := View{width: 0 height: 1}
                fold_open := TreeFold{visible: false draw_icon.svg: crate_resource("self:resources/icons/chevron-down.svg")}
                fold_closed := TreeFold{visible: false draw_icon.svg: crate_resource("self:resources/icons/chevron-right.svg")}
                file_gap := View{width: 20 height: 1}
                heading := ButtonFlatter{width: Fill height: 30 padding: Inset{left: 4 right: 4 top: 0 bottom: 0} align: Align{x: 0.0 y: 0.5} text: "文档" draw_text +: {color: #x3f4650 color_hover: #x111827 text_style +: {font_size: 12}}}
            }
            Empty := View{width: Fill height: 1}
        }
    }
    let DocTab = RectView{visible: false width: Fit height: 30 flow: Right align: Align{x: 0.0 y: 0.5} padding: Inset{left: 6 right: 2} draw_bg +: {color: #x00000000 border_radius: 6.0 border_size: 0.0}
        title := ButtonFlatter{width: Fit height: 28 padding: Inset{left: 6 right: 6 top: 0 bottom: 0} text: "文档" draw_text +: {color: #x4b5563 color_hover: #x111827 text_style +: {font_size: 12}}}
        close := TreeFold{draw_icon.svg: crate_resource("self:resources/icons/close.svg")}
    }
    let SwitchRow = ButtonFlatter{visible: false width: Fill height: 30 padding: Inset{left: 12 right: 12 top: 0 bottom: 0} align: Align{x: 0.0 y: 0.5} text: "" draw_text +: {color: #x303740 color_hover: #x7c3aed text_style +: {font_size: 13}}}
    let FontRow = SolidView{visible: false width: Fill height: 30 flow: Right spacing: 6 align: Align{x: 0.0 y: 0.5} padding: Inset{left: 8 right: 8} draw_bg.color: #xf8f9fb
        Label{text: "☰" draw_text +: {color: #xb6bdc7 text_style +: {font_size: 12}}}
        font_name := Label{width: Fill height: Fit text: "" draw_text +: {color: #x303740 text_style +: {font_size: 13}}}
        font_up := ButtonFlatter{visible: false width: 26 height: 22 text: "↑" draw_text +: {color: #x6b7280 color_hover: #x2162c2 text_style +: {font_size: 12}}}
        font_down := ButtonFlatter{visible: false width: 26 height: 22 text: "↓" draw_text +: {color: #x6b7280 color_hover: #x2162c2 text_style +: {font_size: 12}}}
        font_remove := ButtonFlatter{width: 26 height: 22 text: "×" draw_text +: {color: #x9aa3af color_hover: #xdc2626 text_style +: {font_size: 13}}}
    }
    let PrefNav = ButtonFlatter{width: Fill height: 30 padding: Inset{left: 8 right: 8 top: 0 bottom: 0} align: Align{x: 0.0 y: 0.5} text: "" draw_text +: {color: #x303740 color_hover: #x2162c2 text_style +: {font_size: 13}}}
    let PrefPage = View{visible: false width: Fill height: Fill flow: Down spacing: 8}
    startup() do #(App::script_component(vm)){
        ui: Root{
            main_window := Window{
                window.title: "DocGenie"
                window.inner_size: vec2(1400, 900)
                window_menu +: {
                    main := mod.widgets.MenuItem.Main{items: [@app_menu]}
                    app_menu := mod.widgets.MenuItem.Sub{name: "DocGenie" items: [@settings, @line2, @quit]}
                    settings := mod.widgets.MenuItem.Item{name: "Settings…" key: KeyCode.Comma enabled: true}
                    quit := mod.widgets.MenuItem.Item{name: "Quit DocGenie" key: KeyCode.KeyQ enabled: true}
                }
                body +: {flow: Overlay spacing: 0
                    // The overlay and page share one window-sized stack.
                    page := View{width: Fill height: Fill flow: Down
                    SolidView{width: Fill height: Fill flow: Right spacing: 1 draw_bg.color: #xe7eaee
                        navigation_panel := SolidView{width: 248 height: Fill flow: Down draw_bg.color: #xf8f9fb
                            View{width: Fill height: 44}
                            View{width: Fill height: Fill flow: Down padding: 12 spacing: 10
                            View{width: Fill height: 32 flow: Right align: Align{x: 0.0 y: 0.5}
                                Label{text: "DocGenie" draw_text +: {color: #x20262e text_style +: {font_size: 16}}}
                                View{width: Fill height: 1}
                            }
                            new_document := Action{width: Fill text: "+  新建文档" align: Align{x: 0.0 y: 0.5}}
                            document_search := Action{width: Fill text: "搜索 / 跳转文档     ⌘O" align: Align{x: 0.0 y: 0.5} draw_text +: {color: #x687381}}
                            View{width: Fill height: 32 flow: Right align: Align{x: 0.0 y: 0.5} spacing: 2
                                Caption{text: "文档"}
                                View{width: Fill height: 1}
                                new_folder := IconAction{draw_icon.svg: crate_resource("self:resources/icons/folder-plus.svg")}
                                rename_node := IconAction{draw_icon.svg: crate_resource("self:resources/icons/edit.svg")}
                                delete_node := IconAction{draw_icon.svg: crate_resource("self:resources/icons/trash.svg")}
                            }
                            document_list := FileTree{}
                            Caption{text: "本地文档库"}
                            }
                        }
                        center_panel := SolidView{width: Fill height: Fill flow: Down draw_bg.color: #xffffff
                            center_toolbar := SolidView{width: Fill height: 44 flow: Right align: Align{x: 0.0 y: 0.5} padding: Inset{left: 8 right: 8 top: 6 bottom: 6} spacing: 3 draw_bg.color: #xf8f9fb
                                toggle_navigation := IconAction{draw_icon.svg: crate_resource("self:resources/icons/sidebar-left.svg")}
                                tab_strip := ScrollXView{width: Fill height: 32 flow: Right spacing: 3
                                    tab0 := DocTab{} tab1 := DocTab{} tab2 := DocTab{} tab3 := DocTab{} tab4 := DocTab{} tab5 := DocTab{}
                                }
                                agent_cancel := IconAction{visible: false draw_icon.svg: crate_resource("self:resources/icons/stop.svg")}
                                retry_save := IconAction{draw_icon.svg: crate_resource("self:resources/icons/retry.svg")}
                                undo_button := IconAction{draw_icon.svg: crate_resource("self:resources/icons/undo.svg")}
                                mode_edit := Action{text: "编辑" grab_key_focus: false}
                                mode_read := Action{visible: false text: "阅读" grab_key_focus: false}
                                preferences_button := IconAction{draw_icon.svg: crate_resource("self:resources/icons/settings.svg")}
                                toggle_comments := IconAction{draw_icon.svg: crate_resource("self:resources/icons/sidebar-right.svg")}
                            }
                            empty_workspace := View{visible: false width: Fill height: Fill flow: Down align: Align{x: 0.5 y: 0.5} spacing: 12
                                Label{text: "没有打开的文档"}
                                empty_new := Action{text: "+ 新建文档"}
                                empty_open := Action{text: "打开文档 · ⌘O"}
                            }
                        article_surface := SolidView{width: Fill height: Fill flow: Down align: Align{x: 0.5 y: 0.0} padding: Inset{left: 24 right: 24 top: 32 bottom: 12} draw_bg.color: #xffffff
                        article_column := View{width: Fill max_width: 760 height: Fill flow: Down spacing: 12
                            View{width: Fill height: 44 flow: Right align: Align{x: 0.0 y: 0.5}
                            title_input := TextInput{width: Fill height: 44 empty_text: "请输入标题"
                                draw_text +: {color: #x054aa6 color_hover: #x054aa6 color_focus: #x054aa6 color_empty: #xc5cad3 text_style +: {font_size: 28}}
                                draw_selection +: {pixel: fn(){let color = #x69afa533; return vec4(color.rgb * color.a, color.a)}}
                                draw_bg +: {color: #xffffff color_empty: #xffffff color_focus: #xffffff color_hover: #xffffff border_size: 0.0}
                                draw_cursor +: {color: #x2563eb}
                                blink_speed: 0.5
                            }
                            }
                            View{width: Fill height: Fit flow: Right spacing: 12
                                author_label := Caption{text: "你"}
                                modified_label := Caption{text: "今天修改" draw_text +: {color: #x9099a3}}
                            }
                            live_panel := View{width: Fill height: Fill
                                live_editor := mod.widgets.LiveEditor{}
                            }
                            preview_panel := View{visible: false width: Fill height: Fill
                                preview := mod.widgets.Reading{}
                            }
                            }
                            View{width: Fill height: Fit flow: Right spacing: 10
                                save_label := Caption{text: "正在准备本地文档…"}
                                View{width: Fill height: 1}
                                revision_label := Caption{text: "Revision 0" draw_text +: {color: #x687381 text_style +: {font_size: 10}}}
                            }
                            status_label := Caption{text: "选中文本后添加评论 · ⌘⇧M" draw_text +: {text_style +: {font_size: 11}}}
                        }
                        }
                        comments_panel := SolidView{width: 340 height: Fill flow: Down draw_bg.color: #xf8f9fb
                            View{width: Fill height: 44}
                            View{width: Fill height: Fill flow: Down padding: 16 spacing: 10
                            View{width: Fill height: Fit flow: Right spacing: 2 padding: Inset{left: 0 right: 0 top: 0 bottom: 4}
                                tab_outline := Action{text: "大纲" grab_key_focus: false}
                                tab_comments := Action{text: "评论" grab_key_focus: false}
                                View{width: Fill height: 1}
                            }
                            outline_body := View{visible: false width: Fill height: Fill flow: Down spacing: 6
                                Label{text: "大纲" draw_text +: {color: #x303740 text_style +: {font_size: 14}}}
                                navigation := Navigation{}
                            }
                            comments_body := View{width: Fill height: Fill flow: Down spacing: 8
                            View{width: Fill height: Fit flow: Right spacing: 8
                                filter_open := Action{text: "未解决 0" grab_key_focus: false}
                                filter_resolved := Action{text: "已解决 0" grab_key_focus: false}
                            }
                            threads := mod.widgets.ThreadList{}
                            threads_empty := Caption{text: "还没有评论。选中文本后添加评论。"}
                            View{width: Fill height: Fit flow: Right spacing: 6
                                comments_label := Label{text: "评论 (0)" draw_text +: {color: #x303740 text_style +: {font_size: 14}}}
                                View{width: Fill height: 1}
                                thread_prev := Action{text: "↑" width: 28 height: 28 grab_key_focus: false
                                    draw_text +: {color: #x626b78 color_hover: #x7c3aed color_down: #x7c3aed}
                                }
                                thread_next := Action{text: "↓" width: 28 height: 28 grab_key_focus: false
                                    draw_text +: {color: #x626b78 color_hover: #x7c3aed color_down: #x7c3aed}
                                }
                                thread_resolve_toggle := Action{text: "解决" width: Fit height: 28 grab_key_focus: false
                                    draw_text +: {color: #x626b78 color_hover: #x7c3aed color_down: #x7c3aed}
                                }
                            }
                            SolidView{width: Fill height: 1 draw_bg.color: #xe7eaee}
                            quote := mod.widgets.CommentQuote{}
                            thread_state := Caption{text: "尚未选择段落"}
                            thread_rebind := Action{text: "将当前选段绑定到此线程"}
                            transcript := mod.widgets.Discussion{}
                            View{width: Fill height: 1 draw_bg.color: #xe8eaed}
                            Label{text: "回复" draw_text +: {color: #x626b78 text_style +: {font_size: 12}}}
                            comment_input := TextInput{width: Fill height: 64 is_multiline: true empty_text: "添加评论，或继续回复…" blink_speed: 0.5
                                draw_bg +: {color: #xffffff color_empty: #xffffff color_hover: #xffffff color_focus: #xffffff border_color: #xdce1e8 border_color_focus: #x2563eb}
                                draw_cursor +: {color: #x2563eb}
                            }
                            View{width: Fill height: Fit flow: Right spacing: 8
                                comment_send := Action{text: "发送评论 / 回复"}
                                comment_new := Action{text: "新评论"}
                            }
                            agent_hint := Caption{text: "Agent 默认关闭 · ⌘, 设置"}
                            }
                        }
                    }
                    }
                    }
                    tab_menu := mod.widgets.TabMenu{}
                    comment_menu := mod.widgets.CommentMenu{}
                    comment_composer := mod.widgets.CommentComposer{}
                    dialog_overlay := View{visible: false width: Fill height: Fill flow: Overlay
                        SolidView{width: Fill height: Fill draw_bg.color: #x00000060}
                        SolidView{width: 380 height: Fit align: Align{x: 0.5 y: 0.2} flow: Down padding: 20 spacing: 12 draw_bg.color: #xffffff
                            dialog_title := Label{text: "" draw_text +: {color: #x20262e text_style +: {font_size: 15}}}
                            dialog_input := TextInput{width: Fill height: 30 empty_text: "名称" blink_speed: 0.5
                                draw_bg +: {color: #xffffff color_focus: #xffffff color_hover: #xffffff border_color: #xdce1e8 border_color_focus: #x7c3aed}
                                draw_cursor +: {color: #x7c3aed}
                            }
                            dialog_status := Caption{text: ""}
                            View{width: Fill height: Fit flow: Right spacing: 8
                                View{width: Fill height: 1}
                                dialog_cancel := Action{text: "取消"}
                                dialog_ok := Action{text: "确定"}
                            }
                        }
                    }
                    switcher_overlay := View{visible: false width: Fill height: Fill flow: Overlay
                        SolidView{width: Fill height: Fill draw_bg.color: #x00000040}
                        SolidView{width: 520 height: Fit align: Align{x: 0.5 y: 0.12} flow: Down padding: 8 spacing: 2 draw_bg.color: #xffffff
                            switcher_input := TextInput{width: Fill height: 34 empty_text: "跳转到文档…" blink_speed: 0.5
                                draw_bg +: {color: #xffffff color_focus: #xffffff color_hover: #xffffff border_color: #xdce1e8 border_color_focus: #x7c3aed}
                                draw_cursor +: {color: #x7c3aed}
                            }
                            sw0 := SwitchRow{}
                            sw1 := SwitchRow{}
                            sw2 := SwitchRow{}
                            sw3 := SwitchRow{}
                            sw4 := SwitchRow{}
                            sw5 := SwitchRow{}
                            sw6 := SwitchRow{}
                            sw7 := SwitchRow{}
                        }
                    }
                    preferences_overlay := View{visible: false width: Fill height: Fill flow: Overlay
                        SolidView{width: Fill height: Fill draw_bg.color: #x000000a0}
                        SolidView{width: 720 height: 460 align: Align{x: 0.5 y: 0.3} flow: Right draw_bg.color: #xffffff
                            SolidView{width: 160 height: Fill flow: Down padding: Inset{left: 12 right: 12 top: 18 bottom: 16} spacing: 2 draw_bg.color: #xf5f4f2
                                Label{text: "设置" draw_text +: {color: #x20262e text_style +: {font_size: 16}}}
                                View{width: Fill height: 10}
                                pref_nav_appearance := PrefNav{text: "外观"}
                                pref_nav_editor := PrefNav{text: "编辑器"}
                                pref_nav_agent := PrefNav{text: "Agent"}
                            }
                            View{width: Fill height: Fill flow: Down padding: Inset{left: 20 right: 20 top: 12 bottom: 16} spacing: 10
                                View{width: Fill height: Fit flow: Right
                                    View{width: Fill height: 1}
                                    preferences_done := Action{text: "完成"}
                                }
                                pref_page_appearance := PrefPage{visible: true
                                    Label{text: "外观" draw_text +: {color: #x20262e text_style +: {font_size: 16}}}
                                    Label{text: "正文字号（8–48，即时生效）" draw_text +: {color: #x626b78 text_style +: {font_size: 12}}}
                                    pref_font_size := TextInput{width: 120 height: 28 empty_text: "14"
                                        draw_bg +: {color: #xffffff color_focus: #xffffff color_hover: #xffffff border_color: #xdce1e8 border_color_focus: #x2563eb}
                                        draw_cursor +: {color: #x2563eb}
                                    }
                                    pref_size_status := Caption{text: ""}
                                    View{width: Fill height: 4}
                                    Label{text: "代码字体（从上往下，第一个已安装的生效）" draw_text +: {color: #x626b78 text_style +: {font_size: 12}}}
                                    View{width: Fill height: Fill flow: Down spacing: 4
                                        font_row0 := FontRow{}
                                        font_row1 := FontRow{}
                                        font_row2 := FontRow{}
                                        font_row3 := FontRow{}
                                        font_row4 := FontRow{}
                                        font_row5 := FontRow{}
                                        font_row6 := FontRow{}
                                        font_row7 := FontRow{}
                                    }
                                    pref_font_add := DropDown{width: 220 height: 30}
                                    pref_font_status := Caption{text: ""}
                                }
                                pref_page_editor := PrefPage{
                                    Label{text: "编辑器" draw_text +: {color: #x20262e text_style +: {font_size: 16}}}
                                    pref_auto_save := CheckBox{text: "启用自动保存（防丢失）"}
                                }
                                pref_page_agent := PrefPage{
                                    Label{text: "Agent" draw_text +: {color: #x20262e text_style +: {font_size: 16}}}
                                    pref_default_auto := CheckBox{text: "默认启用 Agent 自动修改（外发段落+评论）"}
                                    View{width: Fill height: 4}
                                    Label{text: "模型（只读）" draw_text +: {color: #x5d6672 text_style +: {font_size: 13}}}
                                    SolidView{width: Fill height: Fit flow: Down padding: Inset{left: 8 right: 8 top: 6 bottom: 6} spacing: 4 draw_bg.color: #xf8f9fb
                                        pref_model_endpoint_label := Caption{text: "Endpoint: …"}
                                        pref_model_name_label := Caption{text: "Model: …"}
                                        pref_model_key_label := Caption{text: "API Key: …"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
