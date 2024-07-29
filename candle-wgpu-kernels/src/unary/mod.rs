/// *********** This File Is Genereted! **********///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Functions{UnaryInplaceContiguous,UnaryFromBufferContiguous,UnaryFromBuffer}
impl crate::EntryPoint for Functions{
    fn get_entry_point(&self) -> &'static str{
        match self{
            Functions::UnaryInplaceContiguous => "gc",Functions::UnaryFromBufferContiguous => "gb",Functions::UnaryFromBuffer => "ga"
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
    