/// *********** This File Is Genereted! **********///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Functions{BinaryBufferFromBuffer,BinaryBufferInplace2ContiguousBoth,BinaryBufferFromBufferContiguousBoth,BinaryBufferInplace1ContiguousBoth}
impl crate::EntryPoint for Functions{
    fn get_entry_point(&self) -> &'static str{
        match self{
            Functions::BinaryBufferFromBuffer => "gy",Functions::BinaryBufferInplace2ContiguousBoth => "gG",Functions::BinaryBufferFromBufferContiguousBoth => "gE",Functions::BinaryBufferInplace1ContiguousBoth => "gF"
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
    