use candle_wgpu_kernels::convert::Functions;

use super::*;

pub fn queue_convert_u32_to_f32(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input: BufferReferenceId,
    input_layout: &crate::Layout,
) -> crate::Result<()> {
    let mut meta = get_meta(&dev);
    meta.add_layout1(&input_layout);

    let pipeline = meta.get_pipeline(Pipelines::Convert(DType::U32, Functions::ConvertToF32));
    let bind_group = create_bind_group_input1(buffer_dest, buffer_input);
    enqueue(
        meta,
        pipeline,
        bind_group,
        input_layout.shape().elem_count() as u32,
        input_layout.shape().elem_count(),
    );
    return Ok(());
}

pub fn queue_convert_u8_to_f32(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input: BufferReferenceId,
    input_layout: &crate::Layout,
) -> crate::Result<()> {
    let mut meta = get_meta(&dev);
    meta.add_layout1(&input_layout);

    let pipeline = meta.get_pipeline(Pipelines::Convert(DType::U8, Functions::ConvertU8ToF32));
    let bind_group = create_bind_group_input1(buffer_dest, buffer_input);
    enqueue(
        meta,
        pipeline,
        bind_group,
        input_layout.shape().elem_count() as u32,
        input_layout.shape().elem_count(),
    );
    return Ok(());
}

pub fn queue_convert_f32_to_u32(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input: BufferReferenceId,
    input_layout: &crate::Layout,
) -> crate::Result<()> {
    let mut meta = get_meta(&dev);
    meta.add_layout1(&input_layout);

    let pipeline = meta.get_pipeline(Pipelines::Convert(DType::F32, Functions::ConvertToU32));

    let bind_group = create_bind_group_input1(buffer_dest, buffer_input);
    enqueue(
        meta,
        pipeline,
        bind_group,
        input_layout.shape().elem_count() as u32,
        input_layout.shape().elem_count(),
    );
    return Ok(());
}

pub fn queue_convert_u32_to_u8(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input: BufferReferenceId,
    start_offset: u32,
    size: u32,
) -> crate::Result<()> {
    let mut meta = get_meta(&dev);
    meta.add(start_offset);
    meta.add(size);

    let pipeline = meta.get_pipeline(Pipelines::Convert(DType::U32, Functions::ConvertU32ToU8));

    let bind_group = create_bind_group_input1(buffer_dest, buffer_input);
    enqueue(
        meta,
        pipeline,
        bind_group,
        ((size + 3) / 4) as u32,
        size as usize,
    );
    return Ok(());
}

pub fn queue_convert_u32_to_i64(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input: BufferReferenceId,
    start_offset: u32,
    size: u32,
) -> crate::Result<()> {
    let mut meta = get_meta(&dev);
    meta.add(start_offset);
    meta.add(size);

    // Get the appropriate pipeline for converting u32 to i64
    let pipeline = meta.get_pipeline(Pipelines::Convert(DType::U32, Functions::ConvertU32ToI64));

    // Create a bind group for the destination and input buffers
    let bind_group = create_bind_group_input1_8(buffer_dest, buffer_input);

    // Calculate the number of workgroups needed.
    // Assuming that each workgroup processes 4 u32 values, and each u32 is converted to an i64
    let num_workgroups = ((size + 3) / 4) as u32;

    // Enqueue the compute operation
    enqueue(
        meta,
        pipeline,
        bind_group,
        num_workgroups, // number of workgroups
        size as usize,  // total size (in elements)
    );

    Ok(())
}

pub fn queue_convert_f32_to_u8(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input: BufferReferenceId,
    start_offset: u32,
    size: u32,
) -> crate::Result<()> {
    let mut meta = get_meta(&dev);
    meta.add(start_offset);
    meta.add(size);

    let pipeline = meta.get_pipeline(Pipelines::Convert(DType::F32, Functions::ConvertF32ToU8));

    let bind_group = create_bind_group_input1(buffer_dest, buffer_input);
    enqueue(
        meta,
        pipeline,
        bind_group,
        ((size + 3) / 4) as u32,
        size as usize,
    );
    return Ok(());
}
