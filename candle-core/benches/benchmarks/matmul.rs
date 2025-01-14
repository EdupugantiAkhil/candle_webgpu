use crate::benchmarks::{BenchDevice, BenchDeviceHandler};
use candle_core::{wgpu::MatmulAlgorithm, DType, Device, Tensor, D};
use criterion::{black_box, criterion_group, BenchmarkId, Criterion, Throughput};
use std::time::{Duration, Instant};
//use tracing_subscriber::{self, fmt::format};

// fn run(a: &Tensor, b: &Tensor) {
//     a.matmul(&b.t().unwrap()).unwrap();
// }

fn run(a: &Tensor, b: &Tensor) {
    a.matmul(&b).unwrap();
}


fn test_matmul(device: &Device, group : &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>, b : usize, m : usize, n : usize, k : usize, is_small_line : bool,size : usize, multiple_sizes : bool, tpa : bool, tpb : bool){
    let dtype = DType::F32;
   

    let lhs;
    if tpa{
        lhs = Tensor::zeros((b, k, m), dtype, device).unwrap().transpose(D::Minus1, D::Minus2).unwrap();
    }
    else{
        lhs = Tensor::zeros((b, m, k), dtype, device).unwrap();
    }

    let rhs;
    if tpb{
        rhs = Tensor::zeros((b, n, k), dtype, device).unwrap().transpose(D::Minus1, D::Minus2).unwrap();
    }
    else{
        rhs = Tensor::zeros((b, k, n), dtype, device).unwrap();
    }


    //let max_iters = (64 * 2048) / size;
    let flops = b * m * n * k;
    group.throughput(Throughput::Bytes(flops as u64));
    group.measurement_time(Duration::from_secs(1));
    group.sample_size(32);
    group.warm_up_time(Duration::from_secs_f32(0.25));
    if device.is_webgpu(){
        if let Device::WebGpu(wgpu) = device{

            let algs;
            if is_small_line{ // we test additional kernels: 
                algs = vec![
                    MatmulAlgorithm::Matmul7,
                    MatmulAlgorithm::Matmul1,


                    MatmulAlgorithm::Matmul16_16,
                    MatmulAlgorithm::Matmul32_32(false, false, true, false),
                    MatmulAlgorithm::Matmul64_64(false, false),
                    MatmulAlgorithm::Matmul64_64_8_8(false, false),
                    MatmulAlgorithm::Matmul128_128(false, false),

                    MatmulAlgorithm::Matmul32_32(false, false, true, false),

                    MatmulAlgorithm::Matmul1_128(false, false, true), 
                    //MatmulAlgorithm::Matmul5_1_128(true, false, true, true),
                    //MatmulAlgorithm::Matmul5_1_128(true, false, true, false),
                    MatmulAlgorithm::Matmul16_64(false, false, true, false),
                    //MatmulAlgorithm::Matmul5_16_64(true, false, true, false),

                    // MatmulAlgorithm::Matmul5_1_128(false, true, true, true),
                    // MatmulAlgorithm::Matmul5_1_128(false, true, true, false), 
                    // MatmulAlgorithm::Matmul5_1_128(true, true, true, true),
                    // MatmulAlgorithm::Matmul5_1_128(true, true, true, false),
                    // MatmulAlgorithm::Matmul5_16_64(false, true, true, false),
                    // MatmulAlgorithm::Matmul5_16_64(true, true, true, false),
                ];
            }
            else{
                algs = vec![
                    MatmulAlgorithm::Matmul64_64_8_8(false, false),
                    MatmulAlgorithm::MatmulX,
                    MatmulAlgorithm::Matmul7,
                    MatmulAlgorithm::Matmul1,
                    MatmulAlgorithm::Matmul16_16,
                    MatmulAlgorithm::Matmul32_32(false, false, true, true),
                    MatmulAlgorithm::Matmul24_24(false,false, true, true),
                    MatmulAlgorithm::Matmul24_48(false,false, true, true),
                    MatmulAlgorithm::Matmul64_64(false, false),
                   
                    MatmulAlgorithm::Matmul64_128(false, false),
                    MatmulAlgorithm::Matmul64_128_8_8(false, false),
                    MatmulAlgorithm::Matmul128_128(false, false),

                    // MatmulAlgorithm::Matmul5_32_32(true, false, true, true),
                    // MatmulAlgorithm::Matmul5_64_64(true, false),
                    // MatmulAlgorithm::Matmul5_64_64_8_8(true, false),
                    // MatmulAlgorithm::Matmul5_128_128(true, false),
                 
                    //MatmulAlgorithm::Matmul5_32_32(true, false, true, true),
                    //MatmulAlgorithm::Matmul5_64_64(true, false),
                    //MatmulAlgorithm::Matmul5_64_64_8_8(true, false),
                    //MatmulAlgorithm::Matmul5_128_128(true, false),  

                    // MatmulAlgorithm::Matmul5_32_32(false, true, true, true),
                    // MatmulAlgorithm::Matmul5_64_64(false, true),
                    // MatmulAlgorithm::Matmul5_64_64_8_8(false, true),
                    // MatmulAlgorithm::Matmul5_128_128(false, true),
                 
                    // MatmulAlgorithm::Matmul5_32_32(true, true, true, true),
                    // MatmulAlgorithm::Matmul5_64_64(true, true),
                    // MatmulAlgorithm::Matmul5_64_64_8_8(true, true),
                    // MatmulAlgorithm::Matmul5_128_128(true, true),  
                   ];
            }
            
           
            for alg in algs {
                *(wgpu.matmul_alg.lock().unwrap()) = alg.clone(); 
                
                let func_name = device.bench_name(format!("matmul_{:?}{}{}", alg, if tpa {"_tranposedA"} else {""}, if tpb {"_tranposedB"} else {""} ));
                //let func_name = device.bench_name("matmul");

                if multiple_sizes{
                    group.bench_with_input( BenchmarkId::new(func_name, size), &size, |b, _| {
                        b.iter_custom(|iters| {
                            let start = Instant::now();
                            for _ in 0..iters {
                                run(black_box(&lhs), black_box(&rhs));
                            }
                            device.sync().unwrap();
                            start.elapsed()
                        })
                    });
                }
                else{
                    group.bench_function(func_name, |b| {
                        b.iter_custom(|iters| {
                            let start = Instant::now();
                            for _ in 0..iters {
                                run(black_box(&lhs), black_box(&rhs));
                            }
                            device.sync().unwrap();
                            start.elapsed()
                        })
                    });
                }
               
            }           
        }
    }
    else{
        let func_name = device.bench_name(format!("matmul{}{}", if tpa {"_tranposedA"} else {""}, if tpb {"_tranposedB"} else {""} ));
        if multiple_sizes{
            group.bench_with_input(BenchmarkId::new(func_name, size),&size,|b, _| {
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    for _i in 0..iters {
                        run(black_box(&lhs), black_box(&rhs));
                    }
                    device.sync().unwrap();
                    start.elapsed()
                })
            });
        }
        else{
            group.bench_function(func_name, |b| {
                b.iter_custom(|iters| {
                    let start = Instant::now();
                    for _ in 0..iters {
                        run(black_box(&lhs), black_box(&rhs));
                    }
                    device.sync().unwrap();
                    start.elapsed()
                })
            });
        }
        
        
    }
}


fn test_functions(device: &Device, group : &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>, fm : impl Fn(usize) -> usize){
    let sizes = vec![1usize, 8, 32, 128, 130, 256,258, 1024,1026, 2048, 2050].into_iter();
    //let sizes = vec![2048u32, 2050].into_iter();
        
    for size in sizes{
        test_matmul(device, group, 1, fm(size), size, size, fm(2) == 1, size, true, false, false);
    }
    // #[cfg(feature = "wgpu_debug")]
    // match device {
    //     candle_core::Device::WebGpu(gpu) => {
    //         let info = pollster::block_on(gpu.get_debug_info()).unwrap();
    //         let map2 = candle_core::wgpu::debug_info::calulate_measurment(&info);
    //         candle_core::wgpu::debug_info::save_list(&map2, "wgpu_bench.json").unwrap();
    //     },
    //     _ => {},
    // };
}

fn criterion_benchmark(c: &mut Criterion) {
    let handler = BenchDeviceHandler::new().unwrap();

    // let mut group = c.benchmark_group("matmul_m_1");
    // for device in handler.devices.iter() {
    //     test_functions(device, &mut group, |_| 1);
    // }
    // group.finish();

    // let mut group = c.benchmark_group("matmul_full");
    // for device in handler.devices.iter() {
    //     test_functions(device, &mut group, |size| size);
    // }
    // group.finish();

    // let mut group = c.benchmark_group("matmul_(2048x2048 * 2048x2048)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 1, 2048, 2048, 2048, false, 1, false, false, false);
    //     test_matmul(device, &mut group, 1, 2048, 2048, 2048, false, 1, false, true, false);
    //     test_matmul(device, &mut group, 1, 2048, 2048, 2048, false, 1, false, false, true);
    //     test_matmul(device, &mut group, 1, 2048, 2048, 2048, false, 1, false, true, true);
    //  }
    // group.finish();

    
    // let mut group = c.benchmark_group("matmul_(32x2304 * 2304x5120)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 1, 32, 5120, 2304, false, 1, false, false, false);
    //     // test_matmul(device, &mut group, 1, 32, 5120, 2304, false, 1, false, true, false);
    //     // test_matmul(device, &mut group, 1, 32, 5120, 2304, false, 1, false, false, true);
    //     // test_matmul(device, &mut group, 1, 32, 5120, 2304, false, 1, false, true, true);
    // }
    // group.finish();

    // let mut group = c.benchmark_group("matmul_(64x2304 * 2304x5120)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 1, 64, 5120, 2304, false, 1, false, false, false);
    //     // test_matmul(device, &mut group, 1, 64, 5120, 2304, false, 1, false, true, false);
    //     // test_matmul(device, &mut group, 1, 64, 5120, 2304, false, 1, false, false, true);
    //     // test_matmul(device, &mut group, 1, 64, 5120, 2304, false, 1, false, true, true);
    // }
    // group.finish();



    // let mut group = c.benchmark_group("matmul_(24x1536 * 1536x6144)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 1, 24, 6144, 1536, false, 1, false, false, false);
    // }
    // group.finish();

     
    // let mut group = c.benchmark_group("matmul_2*(653x1536 * 1536x1536)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 2, 653, 1536, 1536, false, 1, false, false, false);
    // }
    // group.finish();

    let mut group = c.benchmark_group("matmul_48*(24x1536 * 1536x6144) ");
    for device in handler.devices.iter() {
        test_matmul(device, &mut group, 48, 24, 6144, 1536, false, 1, false, false, false);
    }
    group.finish();

    // let mut group = c.benchmark_group("matmul_32*(32x2304 * 2304x5120)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 32, 32, 5120, 2304, false, 1, false, false, false);
    // }
    // group.finish();


    // let mut group = c.benchmark_group("matmul_(1101x1280 * 1280x1280)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 1, 1101, 1280, 1280, false, 1, false, false, false);
    // }
    // group.finish();


    // let mut group = c.benchmark_group("matmul_10*(4096x64 * 64x4173)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 10, 4096, 4173, 64, false, 1, false, false, false);
    // }
    // group.finish();

    
    // let mut group = c.benchmark_group("matmul_20*(1024x64 * 64x1101)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group,  20, 1024, 1101, 64, false, 1, false, false, false);
    // }
    // group.finish();

    // let mut group = c.benchmark_group("matmul_64*(64x1664 * 1664x2560)");
    // for device in handler.devices.iter() {
    //     test_matmul(device, &mut group, 64, 64, 2560, 1664, false, 1, false);
    // }
    // group.finish();
}

criterion_group!(benches, criterion_benchmark);
