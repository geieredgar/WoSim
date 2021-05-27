use std::sync::Arc;

use vulkan::{
    AllocationCreateFlags, AllocationCreateInfo, ComponentMapping, DescriptorImageInfo,
    DescriptorType, Device, Extent2D, Extent3D, Filter, Image, ImageAspectFlags, ImageCreateInfo,
    ImageLayout, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageView,
    ImageViewCreateFlags, ImageViewType, MemoryPropertyFlags, MemoryUsage, SampleCountFlags,
    Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode, SamplerReductionMode,
    SamplerReductionModeCreateInfo, Semaphore, WriteDescriptorSet,
};

use crate::{renderer::RenderConfiguration, view::View};

use super::DepthContext;

pub struct DepthView {
    pub pyramid_views: Vec<ImageView>,
    pub pyramid_view: ImageView,
    pub pyramid_image: Image,
    pub image_view: ImageView,
    pub image: Image,
    pub ready: Semaphore,
    pub sampler: Sampler,
    pub mip_levels: u32,
}

impl DepthView {
    pub fn new(
        device: &Arc<Device>,
        context: &DepthContext,
        configuration: &RenderConfiguration,
        image_extent: Extent2D,
        mip_levels: u32,
    ) -> Result<Self, vulkan::Error> {
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
        let allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::GpuOnly,
            flags: AllocationCreateFlags::empty(),
            required_flags: MemoryPropertyFlags::empty(),
            preferred_flags: MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            pool: None,
            user_data: None,
        };
        let create_info = ImageCreateInfo::builder()
            .extent(Extent3D {
                width: image_extent.width,
                height: image_extent.height,
                depth: 1,
            })
            .tiling(ImageTiling::OPTIMAL)
            .image_type(ImageType::TYPE_2D)
            .usage(ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | ImageUsageFlags::SAMPLED)
            .initial_layout(ImageLayout::UNDEFINED)
            .mip_levels(1)
            .array_layers(1)
            .samples(SampleCountFlags::TYPE_1)
            .format(configuration.depth_format);
        let (image, _) = device.create_image(&create_info, &allocation_info)?;
        let image_view = device.create_image_view(
            ImageViewCreateFlags::empty(),
            &image,
            ImageViewType::TYPE_2D,
            configuration.depth_format,
            ComponentMapping::default(),
            ImageSubresourceRange::builder()
                .aspect_mask(ImageAspectFlags::DEPTH)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build(),
        )?;
        let create_info = ImageCreateInfo::builder()
            .extent(Extent3D {
                width: image_extent.width,
                height: image_extent.height,
                depth: 1,
            })
            .tiling(ImageTiling::OPTIMAL)
            .image_type(ImageType::TYPE_2D)
            .usage(
                ImageUsageFlags::SAMPLED | ImageUsageFlags::STORAGE | ImageUsageFlags::TRANSFER_DST,
            )
            .initial_layout(ImageLayout::UNDEFINED)
            .mip_levels(mip_levels)
            .array_layers(1)
            .samples(SampleCountFlags::TYPE_1)
            .format(configuration.depth_pyramid_format);
        let allocation_info = AllocationCreateInfo {
            usage: MemoryUsage::GpuOnly,
            flags: AllocationCreateFlags::empty(),
            required_flags: MemoryPropertyFlags::empty(),
            preferred_flags: MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            pool: None,
            user_data: None,
        };
        let (pyramid_image, _) = device.create_image(&create_info, &allocation_info)?;
        let pyramid_view = device.create_image_view(
            ImageViewCreateFlags::empty(),
            &pyramid_image,
            ImageViewType::TYPE_2D,
            configuration.depth_pyramid_format,
            ComponentMapping::default(),
            ImageSubresourceRange::builder()
                .aspect_mask(ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(mip_levels)
                .base_array_layer(0)
                .layer_count(1)
                .build(),
        )?;
        let mut pyramid_views = Vec::with_capacity(mip_levels as usize);
        let mut image_infos = Vec::with_capacity(mip_levels as usize);
        let mut descriptor_writes = Vec::with_capacity(View::MAX_MIP_LEVELS * 2);
        assert!(mip_levels > 0);
        for i in 0..mip_levels {
            let pyramid_view = device.create_image_view(
                ImageViewCreateFlags::empty(),
                &pyramid_image,
                ImageViewType::TYPE_2D,
                configuration.depth_pyramid_format,
                ComponentMapping::default(),
                ImageSubresourceRange::builder()
                    .aspect_mask(ImageAspectFlags::COLOR)
                    .base_mip_level(i)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            )?;
            let dst_view = *pyramid_view;
            pyramid_views.push(pyramid_view);
            let src_layout = if i == 0 {
                ImageLayout::SHADER_READ_ONLY_OPTIMAL
            } else {
                ImageLayout::GENERAL
            };
            let src_view = if i == 0 {
                *image_view
            } else {
                *pyramid_views[(i - 1) as usize]
            };
            image_infos.push([
                DescriptorImageInfo::builder()
                    .image_layout(src_layout)
                    .image_view(src_view)
                    .sampler(*sampler)
                    .build(),
                DescriptorImageInfo::builder()
                    .image_layout(ImageLayout::GENERAL)
                    .image_view(dst_view)
                    .build(),
            ])
        }
        for i in 0..mip_levels {
            descriptor_writes.push(
                WriteDescriptorSet::builder()
                    .dst_set(*context.descriptor_sets[i as usize])
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&image_infos[i as usize][0..1])
                    .build(),
            );
            descriptor_writes.push(
                WriteDescriptorSet::builder()
                    .dst_set(*context.descriptor_sets[i as usize])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(DescriptorType::STORAGE_IMAGE)
                    .image_info(&image_infos[i as usize][1..2])
                    .build(),
            );
        }
        for i in mip_levels..View::MAX_MIP_LEVELS as u32 {
            descriptor_writes.push(
                WriteDescriptorSet::builder()
                    .dst_set(*context.descriptor_sets[i as usize])
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&image_infos[mip_levels as usize - 1][0..1])
                    .build(),
            );
            descriptor_writes.push(
                WriteDescriptorSet::builder()
                    .dst_set(*context.descriptor_sets[i as usize])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(DescriptorType::STORAGE_IMAGE)
                    .image_info(&image_infos[mip_levels as usize - 1][1..2])
                    .build(),
            );
        }
        device.update_descriptor_sets(&descriptor_writes, &[]);
        let ready = device.create_semaphore()?;
        Ok(Self {
            pyramid_views,
            pyramid_view,
            pyramid_image,
            image_view,
            image,
            ready,
            sampler,
            mip_levels,
        })
    }
}
