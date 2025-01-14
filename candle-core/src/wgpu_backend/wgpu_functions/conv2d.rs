use candle_wgpu_kernels::conv2d::Functions;
use candle_wgpu_kernels::conv1d::Functions as Functions1d;
use copy::queue_copy4d_padded;
use matmul::SGEMMParams;

use crate::Shape;

use super::*;

pub fn queue_conv2d(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input1: BufferReferenceId,
    buffer_input2: BufferReferenceId,
    dtype: crate::DType,
    params: &crate::conv::ParamsConv2D,
    input_layout: &crate::Layout,
    kernel_layout: &crate::Layout,
) -> crate::Result<()> {
    //if input stride_x is not 1, performance can be extremly bad! -> copy strided
    let input_stride = input_layout.stride();
    let kernel_stride = kernel_layout.stride();
    

    //check if we might use a matrix multiplication instead of convolution:
    if params.k_h == 1 && params.k_w == 1 && input_stride[2] == input_stride[3] * params.i_w  && params.padding == 0 && params.dilation == 1 && params.stride == 1{
        let m = params.c_out;
        let k = params.c_in;
        let n = params.i_h * params.i_w; 

        let new_kernel_layout: Layout = Layout::new(Shape::from_dims(&[params.b_size, m, k]), vec![0, kernel_stride[0], kernel_stride[1]] , kernel_layout.start_offset()); //batch kernel stride is 0, so we will reuse the same kernel for multiple batches
        let new_input_layout: Layout = Layout::new(Shape::from_dims(&[params.b_size, k, n]), vec![input_stride[0],input_stride[1], input_stride[3]] , input_layout.start_offset());
    
        queue_matmul_buffer(dev, buffer_dest, buffer_input2, buffer_input1, SGEMMParams::new(params.b_size, m, k, n), 0 as u32, &new_kernel_layout, &new_input_layout, dtype)?;

        // for b_val in 0..params.b_size{
        //     let new_kernel_layout: Layout = Layout::new(Shape::from_dims(&[1, m, k]), vec![kernel_stride[0], kernel_stride[0], kernel_stride[1]] , kernel_layout.start_offset());
        //     let new_input_layout: Layout = Layout::new(Shape::from_dims(&[1, k, n]), vec![input_stride[1],input_stride[1], input_stride[3]] , input_layout.start_offset() + b_val * input_stride[0]);
    
        //     queue_matmul_buffer(dev, buffer_dest, buffer_input2, buffer_input1, SGEMMParams::new(1, m, k, n), (b_val * params.i_h * params.i_w * params.c_out) as u32, &new_kernel_layout, &new_input_layout, dtype)?;
        // } 

        return Ok(())
    }

    //kernel is contiues in k_h, k_w, c_in -> we might use im2col:
    //this is way faster, but also needs way more memory: 
    if kernel_stride[2] == params.k_w && kernel_stride[1] == params.k_h * params.k_w{
        let mem_needed = 4 * params.c_in * params.k_h * params.k_w * params.b_size * params.out_h() * params.out_w();
        if mem_needed < dev.device_limits.max_storage_buffer_binding_size as usize{
            return queue_conv2d_matmul(dev, buffer_dest, buffer_input1, buffer_input2, dtype, params, input_layout, kernel_layout);
        }
    }

    let mut use_padded = false;

    const MAY_PAD_INPUT : bool = true;

    let is_continues_in_c_in = input_stride[1] == 1;
 
    let (input_buffer, input_layout) = 
    if MAY_PAD_INPUT && params.padding > 0{
        use_padded = true;
        let current_shape = input_layout.shape().dims4()?;
        let padded_shape = (current_shape.0, current_shape.1, current_shape.2 + params.padding * 2, current_shape.3 + params.padding * 2);
        let new_layout = Layout::contiguous_with_offset(Shape::from(padded_shape), 0);
        
        let mut cache = dev.cache.lock().unwrap();
        let tmp_buffer = cache.create_buffer_reference(new_layout.shape().elem_count() * 4, false);
        drop(cache);
        queue_copy4d_padded(dev, tmp_buffer, buffer_input1.clone(), dtype, input_layout, params.padding, &new_layout)?;
        
        (tmp_buffer, new_layout)
    }
    else{
        //the performance is bad if the input is not contiguous
        if input_stride[3] != 1 && (params.c_out > 32) && (params.i_h >= 64 && params.i_w >= 64) {
            let mut cache = dev.cache.lock().unwrap();
            let tmp_buffer = cache.create_buffer_reference(input_layout.shape().elem_count()*4, false);
            
            queue_copy_strided(dev, tmp_buffer.clone(), buffer_input1.clone(), dtype, input_layout, 0)?;
            (tmp_buffer, Layout::contiguous(input_layout.shape()))
        }
        else{
            (buffer_input1.clone(),input_layout.clone())
        }
    };
  

    let padding = if use_padded{
        0
    }
    else{
        params.padding
    };

    let input_stride = input_layout.stride();
    let kernel_stride = kernel_layout.stride();
    
    let mut meta = get_meta(&dev);
 
    let const_vec = vec![
        kernel_stride[3],//kernel_x_stride
        input_stride[3], //stride_x_in
        params.dilation,
        params.k_w, 
        params.k_h,
        params.b_size,
        params.c_in,
        params.i_w,
        params.i_h
        ];

    meta.add(input_layout.start_offset());
    meta.add(kernel_stride[2]); //kernel_y_stride
    meta.add(kernel_stride[1]); //kernel_c_stride
    meta.add(kernel_stride[0]); //kernel_b_stride
    meta.add(kernel_layout.start_offset());
    meta.add(params.i_w);   //size_in_x
    meta.add(params.i_h);   //size_in_y
    meta.add(params.out_w() * params.out_h() * params.c_out); //Stride_batch_out
    meta.add(params.out_w() * params.out_h()); //stride_c_out
    meta.add(params.out_w()); //stride_y_out
    meta.add(params.out_h()); //size_y_out

    meta.add(input_stride[0]); //stride_batch_input
    meta.add(input_stride[1]); //stride_c_in
    meta.add(input_stride[2]); //stride_y_in
    meta.add(padding);
    meta.add(params.stride);
    meta.add(params.c_out);

    //let mut use_channels2 = false;
    let pipeline_function = 
    if is_continues_in_c_in && params.c_in >= 64{
        if padding == 0{
            Functions::Conv2dLongchannelNopadding
        }
        else{
            Functions::Conv2dLongchannel
        }
        
    }
    else if params.k_h == 1 && params.k_w == 1{
        if padding == 0{
            Functions::Conv2dKernelSize1Nopadding
        }
        else{
            Functions::Conv2dKernelSize1
        }
    }
    else if padding == 0{
        Functions::Conv2dNopadding
    }
    else {
        Functions::Conv2d
    };

    let pipeline = meta.get_pipeline_const(Pipelines::Conv2d(get_dtype(dtype)?, pipeline_function), const_vec);

    let bind_group = create_bind_group_input2(
        buffer_dest,
        input_buffer,
        buffer_input2,
    );

    // if use_channels2
    // {
    //     enqueue_workgroups_extra(
    //         meta,
    //         pipeline, 
    //         bind_group,
    //         ((params.c_in + 63) / 64) as u32,
    //         (params.out_w() * params.out_h()) as u32,
    //         ((params.c_out * params.b_size) as u32 + 3)/ 4,
    //         params.out_w() * params.out_h() * params.c_out * params.b_size * kernel_layout.shape().elem_count(),
    //         #[cfg(feature="wgpu_debug")]
    //         Some(format!("{:?}, input1: ({:?}, {:?}), kernel: ({:?}, {:?})", params, input_layout.shape(), input_layout.stride(), kernel_layout.shape(), kernel_layout.stride()))
    //     );
    // }
    // else{
    enqueue_workgroups_extra(
        meta,
        pipeline,
        bind_group,
        (params.out_w() as u32 + 15) / 16,
        (params.out_h() as u32 + 15) / 16,
        (params.c_out * params.b_size) as u32,
        params.out_w() * params.out_h() * params.c_out * params.b_size * kernel_layout.shape().elem_count(),
        #[cfg(feature="wgpu_debug")]
        Some(format!("{:?}, input1: ({:?}, {:?}), kernel: ({:?}, {:?})", params, input_layout.shape(), input_layout.stride(), kernel_layout.shape(), kernel_layout.stride()))
    );
    //}

   
    return Ok(());
}


//calculated conv2d(uses im2col and matmul)
//+ fast(matmul)
//-im2col creates much more memory
pub fn queue_conv2d_matmul(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input1: BufferReferenceId,
    buffer_input2: BufferReferenceId,
    dtype: crate::DType,
    params: &crate::conv::ParamsConv2D,
    input_layout: &crate::Layout,
    kernel_layout: &crate::Layout,
) -> crate::Result<()> {
    //1. im2col
    // Calculate output dimensions
    let o_h = params.out_h();
    let o_w = params.out_w();

    // Get strides from the layouts
    let src_stride = input_layout.stride();
    let kernel_stride = kernel_layout.stride();

    if kernel_stride[2] != params.k_w || kernel_stride[1] != params.k_h * params.k_w{
        panic!("kernel is not contigues in c_in, k_h, k_w")
    }

    let dst_numel = params.k_h * params.k_w * params.b_size * params.c_in * o_h * o_w;

    let const_vec = vec![
        params.padding,
        params.stride,
        params.dilation,
        params.k_h, 
        params.k_w,
        input_layout.start_offset()
        ];

    let mut meta = get_meta(&dev);
    meta.add(dst_numel); // op_conv2d_dst_numel
    meta.add(o_h);                                      // op_conv2d_h_out
    meta.add(o_w);                                      // op_conv2d_w_out
    meta.add(params.c_in);                              // op_conv2d_c_in
    meta.add(params.i_h);                               // op_conv2d_h_in
    meta.add(params.i_w);                               // op_conv2d_w_in
    meta.add(src_stride[0] as u32);                     // op_conv2d_src_s0 (batch stride)
    meta.add(src_stride[1] as u32);                     // op_conv2d_src_s1 (channel stride)
    meta.add(src_stride[2] as u32);                     // op_conv2d_src_s2 (height stride)
    meta.add(src_stride[3] as u32);                     // op_conv2d_src_s3 (width stride) 

   // Dispatch the convolution kernel
   let workgroup_size = 256; // Assumed workgroup size, adjust based on hardware
   let num_workgroups = (dst_numel + workgroup_size - 1) / workgroup_size;

    let b = params.b_size;
    let n = o_h * o_w;
    let m: usize = params.c_out;
    let k = params.c_in * params.k_h * params.k_w;
    let im2col_layout = Layout::new(Shape::from_dims(&[b, k, m]), vec![k * n, 1, k], 0);
    
    let im2col_buffer;
    let pipeline = meta.get_pipeline_const(Pipelines::Conv2d(get_dtype(dtype)?, Functions::Im2col), const_vec);
    {
        let mut cache = dev.cache.lock().unwrap();
    
        im2col_buffer = cache.create_buffer_reference(4 * n * k * b, false);
        
        let bind_group = create_bind_group_input1(
            im2col_buffer,
            buffer_input1);
    
        let x = num_workgroups.min(65535);
        let y = (num_workgroups + 65534) / 65535;

        enqueue_workgroups_extra(
            meta,
            pipeline,
            bind_group,
            x as u32,
            y as u32, 
            1,
            dst_numel,
            #[cfg(feature="wgpu_debug")]
            Some(format!("{:?}, input1: ({:?}, {:?}, {}), kernel: ({:?}, {:?}, {})", params, input_layout.shape(), input_layout.stride(), input_layout.start_offset(), kernel_layout.shape(), kernel_layout.stride(), kernel_layout.start_offset(),))
        );
    
    }
    
    let flattened_kernel_layout = Layout::new(Shape::from_dims(&[1,params.c_out, params.k_h*params.k_w*params.c_in]), vec![0,kernel_stride[0],kernel_stride[3]], kernel_layout.start_offset());
    queue_matmul_buffer(
        dev,
        buffer_dest,          // The final output buffer
        buffer_input2,        // The kernel as input2
        im2col_buffer,        // Result from im2col as input1
        SGEMMParams::new(params.b_size, m, k, n),
        0,
        &flattened_kernel_layout,        // Layout for kernel buffer
        &im2col_layout,       // Layout for im2col buffer
        dtype
    )?;

    return Ok(());
}

pub fn queue_conv2d_transpose(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input1: BufferReferenceId,
    buffer_input2: BufferReferenceId,
    dtype: crate::DType,
    params: &crate::conv::ParamsConvTranspose2D,
    input_layout: &crate::Layout,
    kernel_layout: &crate::Layout,
) -> crate::Result<()> {
    let input_stride = input_layout.stride();
    let kernel_stride = kernel_layout.stride();
    
    let mut meta = get_meta(&dev);
    
    let const_vec = vec![
        kernel_stride[3],//kernel_x_stride
        input_stride[3], //stride_x_in
        params.dilation,
        params.k_w,
        params.k_h,
        params.b_size,
        params.c_in,
        params.i_w,
        params.i_h
        ];
    
    meta.add(input_layout.start_offset());
    meta.add(kernel_stride[2]); //kernel_y_stride
    meta.add(kernel_stride[0]); //kernel_c_stride
    meta.add(kernel_stride[1]); //kernel_b_stride
    meta.add(kernel_layout.start_offset());
    meta.add(params.i_w);   //size_in_x
    meta.add(params.i_h);   //size_in_y
    meta.add(params.out_w() * params.out_h() * params.c_out); //Stride_batch_out
    meta.add(params.out_w() * params.out_h()); //stride_c_out
    meta.add(params.out_w()); //stride_y_out
    meta.add(params.out_h()); //size_y_out

    meta.add(input_stride[0]); //stride_batch_input
    meta.add(input_stride[1]); //stride_c_in
    meta.add(input_stride[2]); //stride_y_in

    meta.add(params.padding);
    meta.add(params.stride);

    let pipeline = meta.get_pipeline_const(Pipelines::Conv2d(get_dtype(dtype)?, Functions::Conv2dTranspose), const_vec);
    let bind_group = create_bind_group_input2(
        buffer_dest,
        buffer_input1,
        buffer_input2,
    );
    enqueue_workgroups(
        meta,
        pipeline,
        bind_group,
        ((params.out_w() - params.output_padding) as u32 + 15) / 16,
        ((params.out_h() - params.output_padding) as u32 + 15) / 16,
        params.c_out as u32,
        params.out_w() * params.out_h() * params.c_out * params.b_size * kernel_layout.shape().elem_count(),
    );
    return Ok(());
}



pub fn queue_conv1d(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input1: BufferReferenceId,
    buffer_input2: BufferReferenceId,
    dtype: crate::DType,
    params: &crate::conv::ParamsConv1D,
    input_layout: &crate::Layout,
    kernel_layout: &crate::Layout,
) -> crate::Result<()> {
    let input_stride = input_layout.stride();
    let kernel_stride = kernel_layout.stride();

    let const_vec = vec![
        kernel_stride[2],//kernel_x_stride
        input_stride[2], //stride_x_in
        params.padding,
        params.stride,
        params.dilation,
        input_layout.start_offset(),
        params.k_size,
        params.b_size,
        params.c_in
        ];
    let mut meta = get_meta(&dev);

    meta.add(params.b_size);
    meta.add(params.c_in);
    meta.add(params.k_size);
    meta.add(kernel_stride[2]); //kernel_x_stride
    meta.add(kernel_stride[1]); //kernel_c_stride
    meta.add(kernel_stride[0]); //kernel_b_stride
    meta.add(kernel_layout.start_offset());
    meta.add(params.l_in);   //size_in_x
    meta.add(params.l_out() * params.c_out); //Stride_batch_out
    meta.add(params.l_out()); //stride_c_out
    meta.add(params.l_out()); //size_y_out

    meta.add(input_stride[0]); //stride_batch_input
    meta.add(input_stride[1]); //stride_c_in
    meta.add(input_stride[2]); //stride_x_in
    meta.add(params.padding);
    meta.add(params.stride);
    meta.add(params.dilation);
    meta.add(input_layout.start_offset());

    let pipeline = meta.get_pipeline_const(Pipelines::Conv1d(get_dtype(dtype)?, Functions1d::Conv1d), const_vec);

    let bind_group = create_bind_group_input2(
        buffer_dest,
        buffer_input1,
        buffer_input2,
    );
    enqueue_workgroups(
        meta,
        pipeline,
        bind_group,
        (params.l_out() as u32 + 63) / 64,
        params.c_out as u32,
        1,
        params.l_out() * params.c_out * params.b_size * kernel_layout.shape().elem_count(),
    );
    return Ok(());
}

pub fn queue_conv1d_transpose(
    dev: &WgpuDevice,
    buffer_dest: BufferReferenceId,
    buffer_input1: BufferReferenceId,
    buffer_input2: BufferReferenceId,
    dtype: crate::DType,
    params: &crate::conv::ParamsConvTranspose1D,
    input_layout: &crate::Layout,
    kernel_layout: &crate::Layout,
) -> crate::Result<()> {
    let input_stride = input_layout.stride();
    let kernel_stride = kernel_layout.stride();
    
    let const_vec = vec![
        kernel_stride[2],//kernel_x_stride
        input_stride[2], //stride_x_in
        params.padding,
        params.stride,
        params.dilation,
        input_layout.start_offset(),
        params.k_size,
        params.b_size,
        params.c_in
        ];
    let mut meta = get_meta(&dev);
    meta.add(params.b_size);
    meta.add(params.c_in);
    meta.add(params.k_size);
    meta.add(kernel_stride[2]); //kernel_x_stride
    meta.add(kernel_stride[0]); //kernel_c_stride
    meta.add(kernel_stride[1]); //kernel_b_stride
    meta.add(kernel_layout.start_offset());
    meta.add(params.l_in);   //size_in_x
    meta.add(params.l_out() * params.c_out); //Stride_batch_out
    meta.add(params.l_out()); //stride_c_out
    meta.add(params.l_out()); //size_y_out

    meta.add(input_stride[0]); //stride_batch_input
    meta.add(input_stride[1]); //stride_c_in
    meta.add(input_stride[2]); //stride_x_in
    meta.add(params.padding);
    meta.add(params.stride);
    meta.add(params.dilation);
    meta.add(input_layout.start_offset());

    let pipeline = meta.get_pipeline_const(Pipelines::Conv1d(get_dtype(dtype)?, Functions1d::Conv1dTranspose), const_vec);
    let bind_group = create_bind_group_input2(
        buffer_dest,
        buffer_input1,
        buffer_input2,
    );
    enqueue_workgroups(
        meta,
        pipeline,
        bind_group,
        ((params.l_out() - params.output_padding) as u32 + 63) / 64,
        params.c_out as u32,
        1u32,
        params.l_out() * params.c_out * params.b_size * kernel_layout.shape().elem_count(),
    );
    return Ok(());
}
