use vkr_core::{ash::vk, *};

mod pipeline;
use pipeline::*;

#[test]
fn simple() {
    let ctx = Ctx::builder().build();
    let mut dev = Dev::new(&ctx, None);
    let pass = Pass::new(&mut dev);
    SimplePipeline::new::<Vertex>(
        &mut dev,
        vk::PrimitiveTopology::TRIANGLE_LIST,
        &pass,
        32,
        32,
    );
    ModelPipeline::new::<Vertex>(
        &mut dev,
        vk::PrimitiveTopology::TRIANGLE_LIST,
        &pass,
        32,
        32,
    );
}
