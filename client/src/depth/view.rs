use std::sync::Arc;

use wosim_common_vulkan::{
    Device, Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode,
    SamplerReductionMode, SamplerReductionModeCreateInfo, VkResult,
};

pub struct DepthView {
    pub sampler: Sampler,
    pub mip_levels: u32,
}

impl DepthView {
    pub fn new(device: &Arc<Device>, mip_levels: u32) -> VkResult<Self> {
        let mut sampler_reduction_info =
            SamplerReductionModeCreateInfo::builder().reduction_mode(SamplerReductionMode::MIN);
        let create_info = SamplerCreateInfo::builder()
            .mag_filter(Filter::LINEAR)
            .min_filter(Filter::LINEAR)
            .mipmap_mode(SamplerMipmapMode::NEAREST)
            .address_mode_u(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(SamplerAddressMode::CLAMP_TO_EDGE)
            .min_lod(0f32)
            .max_lod(mip_levels as f32)
            .push_next(&mut sampler_reduction_info);
        let sampler = device.create_sampler(&create_info)?;
        Ok(Self {
            sampler,
            mip_levels,
        })
    }
}
