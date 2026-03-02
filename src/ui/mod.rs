pub mod helpers;
pub mod tabs;
use adw::prelude::*;
use adw::{ActionRow, ComboRow, HeaderBar, ViewStack, ViewSwitcher};
use gtk4::{Box as GtkBox, Button, Orientation, Revealer, Scale, Switch, ToggleButton};
use libadwaita::{self as adw, ExpanderRow};

pub struct MainWidgets {
    pub window: adw::ApplicationWindow,
    pub status_label: gtk4::Label,
    pub info_model: ActionRow,
    pub row_hz: ActionRow,
    pub row_ctrl: ActionRow,
    pub row_firm: ActionRow,
    pub row_usage: ActionRow,
    pub sv: Scale,
    pub sw_mute: Switch,
    pub btn_off: Button,
    pub btn_ps_off: ToggleButton,
    pub btn_ps_l1: ToggleButton,
    pub btn_ps_l2: ToggleButton,
    pub btn_pl_off: ToggleButton,
    pub btn_pl_l1: ToggleButton,
    pub btn_pl_l2: ToggleButton,
    pub btn_pl_l3: ToggleButton,
    pub combo_input: ComboRow,
    pub combo_output_range: ComboRow,
    pub sw_quick_boot: Switch,
    pub combo_lang: ComboRow,
    pub s_osd_time: Scale,
    pub s_osd_hpos: Scale,
    pub s_osd_vpos: Scale,
    pub s_osd_trans: Scale,
    pub btn_reset_factory: Button,
    pub btn_reset_br_con: Button,
    pub btn_reset_color: Button,
    pub combo_mode: ComboRow,
    pub btn_default: ToggleButton,
    pub btn_custom: ToggleButton,
    pub custom_revealer: Revealer,
    pub sb: Scale,
    pub sc: Scale,
    pub ss: Scale,
    pub sshb: Scale,
    pub sce: Scale,
    pub scoe: Scale,
    pub ssr: Scale,
    pub slbl: Scale,
    pub combo_temp: ComboRow,
    pub sr: Scale,
    pub sg: Scale,
    pub sbl: Scale,
    pub c_hdr: ComboRow,
    pub c_gam: ComboRow,
    pub c_nv: ComboRow,
    pub c_od: ComboRow,
    pub hue_scales: Vec<Scale>,
    pub sat_scales: Vec<Scale>,
    pub exp_size: ExpanderRow,
}

pub fn build_ui(app: &adw::Application) -> MainWidgets {
    let stack = ViewStack::new();
    let header = HeaderBar::builder()
        .decoration_layout(":minimize,maximize,close")
        .build();
    let switcher = ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();
    header.set_title_widget(Some(&switcher));

    let (
        p_main,
        sv,
        sw_m,
        im,
        rh,
        rc,
        rf,
        ru,
        bo,
        bps0,
        bps1,
        bps2,
        bpl0,
        bpl1,
        bpl2,
        bpl3,
        ci,
        cr,
        swb,
        cl,
        st,
        sh,
        sv_osd,
        str_osd,
        b1,
        b2,
        b3,
    ) = tabs::tab_main::build();
    let (
        p_pre,
        cm,
        bdef,
        bcust,
        rev,
        sb,
        sc,
        ss,
        sshb,
        sce,
        scoe,
        ssr,
        slbl,
        ct,
        sr,
        sg,
        sbl,
        chdr,
        cgam,
        cnv,
        cod,
        h_v,
        s_v,
    ) = tabs::tab_presets::build();
    let (
        p_gam,
        _e_size,
        _c_size,
        _e_hz,
        _c_hz_p,
        _e_cross,
        _c_cr_s,
        _c_cr_c,
        _e_stop,
        _c_st_t,
        _c_st_p,
        _e_gt,
        _c_gt_t,
        _c_gt_p,
        _e_mag,
        _s_mag_n,
        _c_mag_z,
        _c_mag_s,
        _c_mag_p,
        _e_align,
        _c_align,
        _e_hawk,
        _c_hawk_s,
        _c_hawk_p,
        _c_hawk_l,
        _sw_a,
        _sw_r,
        _c_dim,
        _c_dyds,
        _c_nv_e,
        _c_od_e,
        _c_hdr_e,
        _s_col_e,
        _s_cr_e,
        _s_sh_e,
        _s_sr_e,
        _s_halo_e,
    ) = tabs::tab_gaming::build();
    let p_inf = tabs::tab_info::build();

    stack.add_titled(&p_main, Some("main"), "Główne");
    stack.add_titled(&p_pre, Some("profiles"), "Profile");
    stack.add_titled(&p_gam, Some("gaming"), "Gaming");
    stack.add_titled(&p_inf, Some("info"), "Info");

    let layout = GtkBox::new(Orientation::Vertical, 0);
    layout.append(&header);
    layout.append(&stack);
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(650)
        .default_height(800)
        .content(&layout)
        .build();

    MainWidgets {
        window,
        status_label: gtk4::Label::new(None),
        info_model: im,
        row_hz: rh,
        row_ctrl: rc,
        row_firm: rf,
        row_usage: ru,
        sv,
        sw_mute: sw_m,
        btn_off: bo,
        btn_ps_off: bps0,
        btn_ps_l1: bps1,
        btn_ps_l2: bps2,
        btn_pl_off: bpl0,
        btn_pl_l1: bpl1,
        btn_pl_l2: bpl2,
        btn_pl_l3: bpl3,
        combo_input: ci,
        combo_output_range: cr,
        sw_quick_boot: swb,
        combo_lang: cl,
        s_osd_time: st,
        s_osd_hpos: sh,
        s_osd_vpos: sv_osd,
        s_osd_trans: str_osd,
        btn_reset_factory: b1,
        btn_reset_br_con: b2,
        btn_reset_color: b3,
        combo_mode: cm,
        btn_default: bdef,
        btn_custom: bcust,
        custom_revealer: rev,
        sb,
        sc,
        ss,
        sshb,
        sce,
        scoe,
        ssr,
        slbl,
        combo_temp: ct,
        sr,
        sg,
        sbl,
        c_hdr: chdr,
        c_gam: cgam,
        c_nv: cnv,
        c_od: cod,
        hue_scales: h_v,
        sat_scales: s_v,
        exp_size: ExpanderRow::new(),
    }
}
