//! Script-created stage objects that are absent from v-slice stage JSON.

use anyhow::Result;
use rustic_asset::{AssetPath, AssetVec2, StageObject};

pub(crate) fn scripted_stage_objects(stage_id: &str) -> Result<Vec<StageObject>> {
    match stage_id {
        "phillyStreets" => Ok(vec![png_object(
            "phillyScrollingSky",
            "images/phillyStreets/phillySkybox.png",
            glam::vec2(-650.0, -375.0),
            glam::vec2(0.65, 0.65),
            glam::vec2(0.1, 0.1),
            10,
            1.0,
        )?]),
        "phillyStreetsErect" => philly_streets_erect_objects(),
        "phillyBlazin" => Ok(vec![png_object(
            "blazinScrollingSky",
            "images/phillyBlazin/skyBlur.png",
            glam::vec2(-700.0, -120.0),
            glam::Vec2::ONE,
            glam::Vec2::ZERO,
            10,
            1.0,
        )?]),
        "limoRideErect" => limo_erect_objects(),
        "tankmanBattlefield" => Ok(vec![png_object(
            "tankCloudsScrolling",
            "images/tankClouds.png",
            glam::vec2(-1100.0, 20.0),
            glam::Vec2::ONE,
            glam::vec2(0.25, 0.25),
            12,
            1.0,
        )?]),
        "sserafim" => sserafim_objects(),
        _ => Ok(Vec::new()),
    }
}

fn philly_streets_erect_objects() -> Result<Vec<StageObject>> {
    Ok(vec![
        png_object(
            "phillyErectScrollingSky",
            "images/phillyStreets/erect/phillySkybox.png",
            glam::vec2(-650.0, -375.0),
            glam::vec2(0.65, 0.65),
            glam::vec2(0.1, 0.1),
            10,
            1.0,
        )?,
        mist_object(
            "phillyMist0",
            "mistMid",
            glam::vec2(1.0, 1.0),
            1.2,
            1000,
            0.6,
        )?,
        mist_object(
            "phillyMist1",
            "mistMid",
            glam::vec2(1.0, 1.0),
            1.1,
            1000,
            0.6,
        )?,
        mist_object(
            "phillyMist2",
            "mistBack",
            glam::vec2(1.0, 1.0),
            1.2,
            1001,
            0.8,
        )?,
        mist_object(
            "phillyMist3",
            "mistMid",
            glam::vec2(0.8, 0.8),
            0.95,
            99,
            0.5,
        )?,
        mist_object(
            "phillyMist4",
            "mistBack",
            glam::vec2(0.7, 0.7),
            0.8,
            88,
            1.0,
        )?,
        mist_object("phillyMist5", "mistMid", glam::vec2(1.1, 1.1), 0.5, 39, 1.0)?,
    ])
}

fn limo_erect_objects() -> Result<Vec<StageObject>> {
    Ok(vec![
        limo_mist_object(
            "limoMist1",
            "mistMid",
            glam::vec2(-650.0, -100.0),
            glam::vec2(1.3, 1.3),
            1.1,
            400,
            0.4,
        )?,
        limo_mist_object(
            "limoMist2",
            "mistBack",
            glam::vec2(-650.0, -100.0),
            glam::vec2(1.0, 1.0),
            1.2,
            401,
            1.0,
        )?,
        limo_mist_object(
            "limoMist3",
            "mistMid",
            glam::vec2(-650.0, -100.0),
            glam::vec2(1.5, 1.5),
            0.8,
            99,
            0.5,
        )?,
        limo_mist_object(
            "limoMist4",
            "mistBack",
            glam::vec2(-650.0, -380.0),
            glam::vec2(1.5, 1.5),
            0.6,
            98,
            1.0,
        )?,
        limo_mist_object(
            "limoMist5",
            "mistMid",
            glam::vec2(-650.0, -400.0),
            glam::vec2(1.5, 1.5),
            0.2,
            15,
            1.0,
        )?,
    ])
}

fn sserafim_objects() -> Result<Vec<StageObject>> {
    Ok(vec![
        sserafim_dust_object(
            "sserafimDust1",
            "dustMid",
            glam::vec2(-650.0, -200.0),
            glam::vec2(1.5, 1.5),
            1.1,
            2000,
            0.8,
        )?,
        sserafim_dust_object(
            "sserafimDust2",
            "dustBack",
            glam::vec2(-650.0, -250.0),
            glam::vec2(1.5, 1.5),
            1.15,
            2000,
            0.9,
        )?,
        sserafim_dust_object(
            "sserafimDust3",
            "dustMid",
            glam::vec2(-650.0, -400.0),
            glam::vec2(2.0, 2.0),
            1.2,
            2000,
            0.8,
        )?,
        sserafim_dust_object(
            "sserafimDust4",
            "dustBack",
            glam::vec2(-650.0, -1300.0),
            glam::vec2(3.5, 3.5),
            1.25,
            2000,
            0.9,
        )?,
    ])
}

fn mist_object(
    id: &str,
    image: &str,
    scale: glam::Vec2,
    scroll: f32,
    z: i32,
    alpha: f32,
) -> Result<StageObject> {
    png_object(
        id,
        &format!("images/phillyStreets/erect/{image}.png"),
        glam::vec2(-650.0, -100.0),
        scale,
        glam::vec2(scroll, scroll),
        z,
        alpha,
    )
}

fn limo_mist_object(
    id: &str,
    image: &str,
    position: glam::Vec2,
    scale: glam::Vec2,
    scroll: f32,
    z: i32,
    alpha: f32,
) -> Result<StageObject> {
    png_object(
        id,
        &format!("images/limo/erect/{image}.png"),
        position,
        scale,
        glam::vec2(scroll, scroll),
        z,
        alpha,
    )
}

fn sserafim_dust_object(
    id: &str,
    image: &str,
    position: glam::Vec2,
    scale: glam::Vec2,
    scroll: f32,
    z: i32,
    alpha: f32,
) -> Result<StageObject> {
    png_object(
        id,
        &format!("images/sserafim/dust/{image}.png"),
        position,
        scale,
        glam::vec2(scroll, scroll),
        z,
        alpha,
    )
}

fn png_object(
    id: &str,
    image: &str,
    position: glam::Vec2,
    scale: glam::Vec2,
    scroll: glam::Vec2,
    z: i32,
    alpha: f32,
) -> Result<StageObject> {
    let mut object = StageObject::png(id, AssetPath::new(image)?);
    object.position = AssetVec2::new(position.x, position.y);
    object.scroll_factor = AssetVec2::new(scroll.x, scroll.y);
    object.scale = AssetVec2::new(scale.x, scale.y);
    object.z = z;
    object.alpha = alpha;
    Ok(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_weekend_scripted_backdrops_from_stage_scripts() {
        let philly = scripted_stage_objects("phillyStreets").unwrap();
        assert_eq!(philly[0].id, "phillyScrollingSky");
        assert_eq!(
            philly[0].image.as_str(),
            "images/phillyStreets/phillySkybox.png"
        );

        let erect = scripted_stage_objects("phillyStreetsErect").unwrap();
        assert_eq!(erect.len(), 7);
        assert!(erect.iter().any(|object| object.id == "phillyMist5"));

        let tank = scripted_stage_objects("tankmanBattlefield").unwrap();
        assert_eq!(tank[0].id, "tankCloudsScrolling");

        let sserafim = scripted_stage_objects("sserafim").unwrap();
        assert_eq!(sserafim.len(), 4);
        assert_eq!(
            sserafim[0].image.as_str(),
            "images/sserafim/dust/dustMid.png"
        );

        let limo = scripted_stage_objects("limoRideErect").unwrap();
        assert_eq!(limo.len(), 5);
        assert_eq!(limo[0].id, "limoMist1");
        assert_eq!(limo[4].position.y, -400.0);
    }
}
