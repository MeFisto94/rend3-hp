use rend3::graph::{NodeExecutionContext, RenderGraph, RenderGraphNodeBuilder};
use rend3::Renderer;
use std::sync::Arc;
use wgpu::{
    BindGroupLayout, ComputePass, ComputePassDescriptor, ComputePipeline, PipelineCompilationOptions,
    PipelineLayoutDescriptor, ShaderModule,
};

pub struct ComputeRoutineArgs<'a, 'node> {
    pub graph: &'a mut RenderGraph<'node>,
    pub label: &'a str,
}

// TODO: a) support in rend3::graph for passes, b) refactor the engine to use this.
pub struct ComputeRoutine<U> {
    pipeline: ComputePipeline,
    bgls: Vec<BindGroupLayout>,
    pub user_data: U,
}

impl<U> ComputeRoutine<U> {
    pub fn new<F>(
        label: &str,
        renderer: &Arc<Renderer>,
        module: &ShaderModule,
        entry_point: &str,
        bgl_builder: F,
        user_data: U,
    ) -> Self
    where
        F: FnOnce(&Arc<Renderer>) -> Vec<BindGroupLayout>,
    {
        let bgls = bgl_builder(renderer);
        // let bgls: Vec<BindGroupLayout> =
        //     builders.iter().map(|(builder, label)| builder.build(&renderer.device, Some(&label))).collect();

        let bgl_borrows: Vec<&BindGroupLayout> = bgls.iter().collect();

        let pll = renderer.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{} pll", label)),
            bind_group_layouts: &bgl_borrows,
            push_constant_ranges: &[],
        });

        let label_str = format!("{} pipeline", label);
        let desc = wgpu::ComputePipelineDescriptor {
            label: Some(&label_str),
            layout: Some(&pll),
            module,
            entry_point: Some(entry_point),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        };

        let pipeline = renderer.device.create_compute_pipeline(&desc);
        Self { bgls, pipeline, user_data }
    }

    /// Add the given routine to the graph with the given settings.
    pub fn add_compute_to_graph<'node, F, G, T>(
        &'node self,
        args: ComputeRoutineArgs<'_, 'node>,
        build_graph: F,
        build_encoder: G,
    ) where
        F: FnOnce(&mut RenderGraphNodeBuilder) -> T,
        G: FnOnce(&mut NodeExecutionContext, &mut ComputePass, &Vec<BindGroupLayout>, T) + 'node,
        T: 'node,
    {
        let mut builder = args.graph.add_node(args.label);
        let t: T = build_graph(&mut builder);
        let label = format!("{} cpass", args.label);

        builder.build(move |mut ctx| {
            let encoder = ctx.encoder_or_pass.take_encoder();
            let mut enc = encoder.borrow_mut();

            // TODO: This can be moved into the builder's add_computepass or similar
            let mut cpass =
                enc.begin_compute_pass(&ComputePassDescriptor { label: Some(&label), timestamp_writes: None });
            cpass.set_pipeline(&self.pipeline);

            build_encoder(&mut ctx, &mut cpass, &self.bgls, t);
            drop(cpass);
        });
    }
}
