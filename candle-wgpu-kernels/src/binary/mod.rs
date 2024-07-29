/// *********** This File Is Genereted! **********///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Functions{BinaryBufferInplace2ContiguousBoth,BinaryBufferInplace1ContiguousBoth,BinaryBufferFromBuffer,BinaryBufferFromBufferContiguousBoth}
impl crate::EntryPoint for Functions{
    fn get_entry_point(&self) -> &'static str{
        match self{
            Functions::BinaryBufferInplace2ContiguousBoth => "gd",Functions::BinaryBufferInplace1ContiguousBoth => "gc",Functions::BinaryBufferFromBuffer => "ga",Functions::BinaryBufferFromBufferContiguousBoth => "gb"
        }
    } 
}
pub fn load_shader(typ : crate::DType) -> &'static str {
    match typ{
        crate::DType::F32 => include_str!("generated/shader.pwgsl_generated_f32.wgsl"),
        crate::DType::U32 => include_str!("generated/shader.pwgsl_generated_u32.wgsl"),
        crate::DType::U8 => include_str!("generated/shader.pwgsl_generated_u8.wgsl"),
    }
}
    