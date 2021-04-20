use std::sync::Arc;

use wosim_common::vulkan::{
    self, AllocationCreateFlags, AllocationCreateInfo, ComponentMapping, DescriptorImageInfo,
    DescriptorPool, DescriptorPoolSetup, DescriptorSet, DescriptorType, Device, Extent2D, Extent3D,
    Image, ImageAspectFlags, ImageCreateInfo, ImageLayout, ImageSubresourceRange, ImageTiling,
    ImageType, ImageUsageFlags, ImageView, ImageViewCreateFlags, ImageViewType,
    MemoryPropertyFlags, MemoryUsage, SampleCountFlags, Semaphore, WriteDescriptorSet,
};

use crate::renderer::RenderConfiguration;

use super::{DepthContext, DepthView};

pub struct DepthFrame {
    pub pyramid_views: Vec<ImageView>,
    pub pyramid_view: ImageView,
    pub pyramid_image: Image,
    pub image_view: ImageView,
    pub image: Image,
    pub descriptor_sets: Vec<DescriptorSet>,
    pub ready: Semaphore,
}

impl DepthFrame {
    pub fn new(
        device: &Arc<Device>,
        context: &DepthContext,
        view: &DepthView,
        configuration: &RenderConfiguration,
        descriptor_pool: &DescriptorPool,
        image_extent: Extent2D,
    ) -> Result<Self, vulkan::Error> {
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
            .mip_levels(view.mip_levels)
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
                .level_count(view.mip_levels)
                .base_array_layer(0)
                .layer_count(1)
                .build(),
        )?;
        let mut pyramid_views = Vec::with_capacity(view.mip_levels as usize);
        let mut set_layouts = Vec::with_capacity(view.mip_levels as usize);
        set_layouts.resize(view.mip_levels as usize, &context.set_layout);
        let descriptor_sets = descriptor_pool.allocate(&set_layouts)?;
        let mut image_infos = Vec::with_capacity(view.mip_levels as usize);
        let mut descriptor_writes = Vec::with_capacity(view.mip_levels as usize * 2);
        for i in 0..view.mip_levels {
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
                    .sampler(*view.sampler)
                    .build(),
                DescriptorImageInfo::builder()
                    .image_layout(ImageLayout::GENERAL)
                    .image_view(dst_view)
                    .build(),
            ])
        }
        for i in 0..view.mip_levels {
            descriptor_writes.push(
                WriteDescriptorSet::builder()
                    .dst_set(*descriptor_sets[i as usize])
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&image_infos[i as usize][0..1])
                    .build(),
            );
            descriptor_writes.push(
                WriteDescriptorSet::builder()
                    .dst_set(*descriptor_sets[i as usize])
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(DescriptorType::STORAGE_IMAGE)
                    .image_info(&image_infos[i as usize][1..2])
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
            descriptor_sets,
            ready,
        })
    }

    pub fn pool_setup(mip_levels: u32) -> DescriptorPoolSetup {
        DescriptorPoolSetup {
            storage_buffers: 0,
            uniform_buffers: 0,
            sets: 1,
            combined_image_samplers: 1,
            storage_images: 1,
        } * mip_levels
    }
}
