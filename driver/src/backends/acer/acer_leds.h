/*!
 * Copyright (c) 2026 Stanly Silas <vstanlysilas@gmail.com>
 *
 * This file is part of acer-keyboard.
 *
 * acer-keyboard is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This software is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this software.  If not, see <https://www.gnu.org/licenses/>.
 */

#ifndef ACER_LEDS_H
#define ACER_LEDS_H

#include <linux/types.h>
#include <linux/platform_device.h>
#include <linux/leds.h>

enum acer_kb_backlight_types {
	ACER_KB_BACKLIGHT_TYPE_NONE = 0x00,
	ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR = 0x01,
	ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB = 0x02,
	ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB = 0x06,
	ACER_KB_BACKLIGHT_TYPE_PER_KEY_RGB = 0xf3
};

int acer_leds_init(struct platform_device *dev);
int acer_leds_remove(struct platform_device *dev);
int acer_leds_suspend(struct platform_device *dev);
int acer_leds_resume(struct platform_device *dev);
enum acer_kb_backlight_types acer_leds_get_backlight_type(void);
void acer_leds_restore_state_extern(void);
void acer_leds_notify_brightness_change_extern(void);
void acer_leds_set_brightness_extern(enum led_brightness brightness);
void acer_leds_set_color_extern(u32 color);

// TODO The following should go into a seperate .c file, but for this to work more reworking is required in the acer_keyboard structure.

#include "acer_leds.h"

#include "acer_interfaces.h"

#include <linux/led-class-multicolor.h>
#include <linux/delay.h>

#define ACER_KBD_BRIGHTNESS_MAX			        0xff
#define ACER_KBD_BRIGHTNESS_DEFAULT			0x00

#define ACER_KBD_BRIGHTNESS_WHITE_MAX			0x02 // White only keyboards can only be off, half, or full brightness
#define ACER_KBD_BRIGHTNESS_WHITE_DEFAULT		0x00

#define ACER_KBD_BRIGHTNESS_WHITE_MAX_5		        0x05 // Devices <= Intel 7th gen had a different white control with 5 brightness values + off
#define ACER_KBD_BRIGHTNESS_WHITE_MAX_5_DEFAULT	        0x00

#define ACER_KB_COLOR_DEFAULT_RED			0xff
#define ACER_KB_COLOR_DEFAULT_GREEN			0xff
#define ACER_KB_COLOR_DEFAULT_BLUE			0xff
#define ACER_KB_COLOR_DEFAULT				((ACER_KB_COLOR_DEFAULT_RED << 16) + (ACER_KB_COLOR_DEFAULT_GREEN << 8) + ACER_KB_COLOR_DEFAULT_BLUE)

static enum acer_kb_backlight_types acer_kb_backlight_type = ACER_KB_BACKLIGHT_TYPE_NONE;
static bool leds_initialized = false;

/**
 * Color scaling quirk list
 */
static void color_scaling(enum acer_kb_backlight_types *type, u8 *red, u8 *green, u8 *blue)
{
	if (*type == ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB) {
		*red = (180 * *red) / 255;
		*blue = (200 * *blue) / 255;
	}
}

static int acer_evaluate_set_white_brightness(u8 brightness)
{
	pr_debug("Set white brightness to %d\n", brightness);

	return acer_evaluate_method (ACER_CMD_SET_KB_WHITE_LEDS, brightness, NULL);
}

static int acer_evaluate_set_rgb_brightness(u8 brightness)
{
	pr_debug("Set RGB brightness to %d\n", brightness);

	return acer_evaluate_method (ACER_CMD_SET_KB_RGB_LEDS, ACER_CMD_SET_KB_LEDS_SUB_RGB_BRIGHTNESS | brightness, NULL);
}

static int acer_evaluate_set_rgb_color(u32 zone, u32 color)
{
	u32 cset = ((color & 0x0000FF) << 16) | ((color & 0xFF0000) >> 8) | ((color & 0x00FF00) >> 8);
	u32 acer_submethod_arg = zone | cset;

	pr_debug("Set Color 0x%08x for region 0x%08x\n", color, zone);

	return acer_evaluate_method(ACER_CMD_SET_KB_RGB_LEDS, acer_submethod_arg, NULL);
}

static int acer_evaluate_set_keyboard_status(u8 state)
{
	u32 cmd = 0xE0000000;
	ACER_INFO("Set keyboard enabled to: %d\n", state);

	if (state == 0) {
		cmd |= 0x003001;
	} else {
		cmd |= 0x07F001;
	}

	return acer_evaluate_method(ACER_CMD_SET_KB_RGB_LEDS, cmd, NULL);
}

static void acer_leds_set_brightness(struct led_classdev *led_cdev __always_unused, enum led_brightness brightness) {
	int ret = acer_evaluate_set_white_brightness(brightness);
	if (ret) {
		pr_debug("acer_leds_set_brightness(): acer_evaluate_set_white_brightness() failed\n");
		return;
	}
	led_cdev->brightness = brightness;
}

/*static void acer_leds_set_brightness_mc(struct led_classdev *led_cdev, enum led_brightness brightness) {
	int ret;
	u32 zone, color;
	struct led_classdev_mc *mcled_cdev = lcdev_to_mccdev(led_cdev);

	ret = acer_evaluate_set_rgb_brightness(ACER_KBD_BRIGHTNESS_MAX);
	if (ret) {
		pr_debug("acer_leds_set_brightness_mc(): acer_evaluate_set_rgb_brightness() failed\n");
		return;
	}

	zone = mcled_cdev->subled_info[0].channel;

	led_mc_calc_color_components(mcled_cdev, brightness);
	color = (mcled_cdev->subled_info[0].brightness << 16) +
		(mcled_cdev->subled_info[1].brightness << 8) +
		mcled_cdev->subled_info[2].brightness;

	ret = acer_evaluate_set_rgb_color(zone, color);
	if (ret) {
		pr_debug("acer_leds_set_brightness_mc(): acer_evaluate_set_rgb_color() failed\n");
		return;
	}
	led_cdev->brightness = brightness;
}*/

// Temprary fix for KDE: KDE does only set one kbd_backlight brightness value, this version of the
// function uses acers built in brightness setting to set the whole keyboard brightness at once.
// -> use acer_evaluate_set_rgb_brightness() to set overall brightness via firmware instead of scaling
//    the RGB values
// -> update all acer_mcled_cdevs brightness levels to refect that the firmware method sets the
//    the whole keyboard brightness and not just one zone
// This is a temporary fix until KDE handles multiple keyboard backlights correctly
static struct led_classdev_mc acer_mcled_cdevs[3]; // forward declaration
static void acer_leds_set_brightness_mc(struct led_classdev *led_cdev, enum led_brightness brightness) {
	int ret;
	u32 zone, color;
	u8 red, green, blue;
	struct led_classdev_mc *mcled_cdev = lcdev_to_mccdev(led_cdev);

	ret = acer_evaluate_set_rgb_brightness(brightness);
	if (ret) {
		pr_debug("acer_leds_set_brightness_mc(): acer_evaluate_set_rgb_brightness() failed\n");
		return;
	}
	acer_mcled_cdevs[0].led_cdev.brightness = brightness;
	acer_mcled_cdevs[1].led_cdev.brightness = brightness;
	acer_mcled_cdevs[2].led_cdev.brightness = brightness;

	zone = mcled_cdev->subled_info[0].channel;

	red = mcled_cdev->subled_info[0].intensity;
	green = mcled_cdev->subled_info[1].intensity;
	blue = mcled_cdev->subled_info[2].intensity;

	color_scaling(&acer_kb_backlight_type, &red, &green, &blue);

	color = (red << 16) +
		(green << 8) +
		blue;

	ret = acer_evaluate_set_rgb_color(zone, color);
	if (ret) {
		pr_debug("acer_leds_set_brightness_mc(): acer_evaluate_set_rgb_color() failed\n");
	}
}

static struct led_classdev acer_led_cdev = {
	.name = "white:" LED_FUNCTION_KBD_BACKLIGHT,
	.max_brightness = ACER_KBD_BRIGHTNESS_WHITE_MAX,
	.brightness_set = &acer_leds_set_brightness,
	.brightness = ACER_KBD_BRIGHTNESS_WHITE_DEFAULT,
	.flags = LED_BRIGHT_HW_CHANGED
};

static struct mc_subled acer_mcled_cdevs_subleds[3][3] = {
	{
		{
			.color_index = LED_COLOR_ID_RED,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_RED,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_0
		},
		{
			.color_index = LED_COLOR_ID_GREEN,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_GREEN,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_0
		},
		{
			.color_index = LED_COLOR_ID_BLUE,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_BLUE,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_0
		}
	},
	{
		{
			.color_index = LED_COLOR_ID_RED,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_RED,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_1
		},
		{
			.color_index = LED_COLOR_ID_GREEN,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_GREEN,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_1
		},
		{
			.color_index = LED_COLOR_ID_BLUE,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_BLUE,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_1
		}
	},
	{
		{
			.color_index = LED_COLOR_ID_RED,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_RED,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_2
		},
		{
			.color_index = LED_COLOR_ID_GREEN,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_GREEN,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_2
		},
		{
			.color_index = LED_COLOR_ID_BLUE,
			.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
			.intensity = ACER_KB_COLOR_DEFAULT_BLUE,
			.channel = ACER_CMD_SET_KB_LEDS_SUB_RGB_ZONE_2
		}
	}
};

static struct led_classdev_mc acer_mcled_cdevs[3] = {
	{
		.led_cdev.name = "rgb:" LED_FUNCTION_KBD_BACKLIGHT,
		.led_cdev.max_brightness = ACER_KBD_BRIGHTNESS_MAX,
		.led_cdev.brightness_set = &acer_leds_set_brightness_mc,
		.led_cdev.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
		.num_colors = 3,
		.subled_info = acer_mcled_cdevs_subleds[0]
	},
	{
		.led_cdev.name = "rgb:" LED_FUNCTION_KBD_BACKLIGHT,
		.led_cdev.max_brightness = ACER_KBD_BRIGHTNESS_MAX,
		.led_cdev.brightness_set = &acer_leds_set_brightness_mc,
		.led_cdev.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
		.num_colors = 3,
		.subled_info = acer_mcled_cdevs_subleds[1]
	},
	{
		.led_cdev.name = "rgb:" LED_FUNCTION_KBD_BACKLIGHT,
		.led_cdev.max_brightness = ACER_KBD_BRIGHTNESS_MAX,
		.led_cdev.brightness_set = &acer_leds_set_brightness_mc,
		.led_cdev.brightness = ACER_KBD_BRIGHTNESS_DEFAULT,
		.num_colors = 3,
		.subled_info = acer_mcled_cdevs_subleds[2]
	}
};

int acer_leds_init(struct platform_device *dev)
{
	int ret, i;
	int status;
	union acpi_object *result;
	u32 result_fallback;

	for (i = 0; i < 3; ++i) {
		status = acer_evaluate_method2(ACER_CMD_GET_SPECS, 0, &result);
		if (!status) {
			if (result->type == ACPI_TYPE_BUFFER) {
				pr_debug("ACER_CMD_GET_SPECS result->buffer.pointer[0x0f]: 0x%02x\n", result->buffer.pointer[0x0f]);
				acer_kb_backlight_type = result->buffer.pointer[0x0f];
				if (acer_kb_backlight_type) {
					status = acer_evaluate_method(ACER_CMD_GET_BIOS_FEATURES_2, 0, &result_fallback);
					if (!status) {
						pr_debug("ACER_CMD_GET_BIOS_FEATURES_2 result_fallback: 0x%08x\n", result_fallback);
						if (result_fallback & ACER_CMD_GET_BIOS_FEATURES_2_SUB_WHITE_ONLY_KB_MAX_5) {
							acer_led_cdev.max_brightness = ACER_KBD_BRIGHTNESS_WHITE_MAX_5;
							acer_led_cdev.brightness = ACER_KBD_BRIGHTNESS_WHITE_MAX_5_DEFAULT;
						}
					}
					break;
				} else {
					pr_debug("acer_kb_backlight_type 0x00 probably wrong, retrying...\n");
					msleep(50);
				}
			}
			else {
				pr_err("ACER_CMD_GET_SPECS does not exist on this device or return value has wrong type, trying ACER_CMD_GET_BIOS_FEATURES\n");
				status = -EINVAL;
			}
			ACPI_FREE(result);
		}
		else {
			pr_notice("ACER_CMD_GET_SPECS does not exist on this device or failed, trying ACER_CMD_GET_BIOS_FEATURES_1\n");
		}
	}

	if (status || acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_NONE) {
		// check for devices <= Intel 8th gen (only white only, 3 zone RGB, or no backlight on these devices)
		status = acer_evaluate_method(ACER_CMD_GET_BIOS_FEATURES_1, 0, &result_fallback);
		if (!status) {
			pr_debug("ACER_CMD_GET_BIOS_FEATURES_1 result_fallback: 0x%08x\n", result_fallback);
			if (result_fallback & ACER_CMD_GET_BIOS_FEATURES_1_SUB_3_ZONE_RGB_KB) {
				acer_kb_backlight_type = ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB;
			}
			else if (result_fallback & ACER_CMD_GET_BIOS_FEATURES_1_SUB_WHITE_ONLY_KB) {
				acer_kb_backlight_type = ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR;

				status = acer_evaluate_method(ACER_CMD_GET_BIOS_FEATURES_2, 0, &result_fallback);
				if (!status) {
					pr_debug("ACER_CMD_GET_BIOS_FEATURES_2 result_fallback: 0x%08x\n", result_fallback);
					if (result_fallback & ACER_CMD_GET_BIOS_FEATURES_2_SUB_WHITE_ONLY_KB_MAX_5) {
						acer_led_cdev.max_brightness = ACER_KBD_BRIGHTNESS_WHITE_MAX_5;
						acer_led_cdev.brightness = ACER_KBD_BRIGHTNESS_WHITE_MAX_5_DEFAULT;
					}
				}
				else {
					pr_notice("ACER_CMD_GET_BIOS_FEATURES_2 does not exist on this device or failed\n");
				}
			}
		}
		else {
			pr_notice("ACER_CMD_GET_BIOS_FEATURES_1 does not exist on this device or failed\n");
		}
	}
	pr_debug("Keyboard backlight type: 0x%02x\n", acer_kb_backlight_type);

	if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR)
		acer_leds_set_brightness_extern(acer_led_cdev.brightness);
	else
		acer_leds_set_color_extern(ACER_KB_COLOR_DEFAULT);

	if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR) {
		pr_debug("Registering fixed color leds interface\n");
		ret = led_classdev_register(&dev->dev, &acer_led_cdev);
		if (ret) {
			pr_err("Registering fixed color leds interface failed\n");
			return ret;
		}
	}
	else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB) {
		acer_evaluate_set_keyboard_status(1);
		pr_debug("Registering single zone rgb leds interface\n");
		ret = devm_led_classdev_multicolor_register(&dev->dev, &acer_mcled_cdevs[0]);
		if (ret) {
			pr_err("Registering single zone rgb leds interface failed\n");
			return ret;
		}
	}
	else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB) {
		acer_evaluate_set_keyboard_status(1);
		pr_debug("Registering three zone rgb leds interface\n");
		ret = devm_led_classdev_multicolor_register(&dev->dev, &acer_mcled_cdevs[0]);
		if (ret) {
			pr_err("Registering three zone rgb zone 0 leds interface failed\n");
			return ret;
		}
		ret = devm_led_classdev_multicolor_register(&dev->dev, &acer_mcled_cdevs[1]);
		if (ret) {
			pr_err("Registering three zone rgb zone 1 leds interface failed\n");
			devm_led_classdev_multicolor_unregister(&dev->dev, &acer_mcled_cdevs[0]);
			return ret;
		}
		ret = devm_led_classdev_multicolor_register(&dev->dev, &acer_mcled_cdevs[2]);
		if (ret) {
			pr_err("Registering three zone rgb zone 2 leds interface failed\n");
			devm_led_classdev_multicolor_unregister(&dev->dev, &acer_mcled_cdevs[0]);
			devm_led_classdev_multicolor_unregister(&dev->dev, &acer_mcled_cdevs[1]);
			return ret;
		}
	}

	leds_initialized = true;
	return 0;
}
EXPORT_SYMBOL(acer_leds_init);

int acer_leds_suspend(struct platform_device *dev)
{
	switch (acer_kb_backlight_type) {
	case ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB:
	case ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB:
		acer_evaluate_set_keyboard_status(0);
		break;
	default:
		break;
	}
	return 0;
}
EXPORT_SYMBOL(acer_leds_suspend);

int acer_leds_resume(struct platform_device *dev)
{
	switch (acer_kb_backlight_type) {
	case ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB:
	case ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB:
		acer_evaluate_set_keyboard_status(1);
		break;
	default:
		break;
	}
	return 0;
}
EXPORT_SYMBOL(acer_leds_resume);

int acer_leds_remove(struct platform_device *dev) {
	if (leds_initialized) {
		if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR) {
			led_classdev_unregister(&acer_led_cdev);
		}
		else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB) {
			devm_led_classdev_multicolor_unregister(&dev->dev, &acer_mcled_cdevs[0]);
		}
		else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB) {
			devm_led_classdev_multicolor_unregister(&dev->dev, &acer_mcled_cdevs[0]);
			devm_led_classdev_multicolor_unregister(&dev->dev, &acer_mcled_cdevs[1]);
			devm_led_classdev_multicolor_unregister(&dev->dev, &acer_mcled_cdevs[2]);
		}
	}

	leds_initialized = false;

	return 0;
}
EXPORT_SYMBOL(acer_leds_remove);

enum acer_kb_backlight_types acer_leds_get_backlight_type(void) {
	return acer_kb_backlight_type;
}
EXPORT_SYMBOL(acer_leds_get_backlight_type);

// TODO Don't reuse brightness_set as it is writing back the same brightness which could lead to race conditions.
// Reimplement brightness_set instead without writing back brightness value like in uniwill_leds.h.
void acer_leds_restore_state_extern(void) {
	if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR) {
		acer_led_cdev.brightness_set(&acer_led_cdev, acer_led_cdev.brightness);
	}
	else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB) {
		acer_mcled_cdevs[0].led_cdev.brightness_set(&acer_mcled_cdevs[0].led_cdev, acer_mcled_cdevs[0].led_cdev.brightness);
	}
	else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB) {
		acer_mcled_cdevs[0].led_cdev.brightness_set(&acer_mcled_cdevs[0].led_cdev, acer_mcled_cdevs[0].led_cdev.brightness);
		acer_mcled_cdevs[1].led_cdev.brightness_set(&acer_mcled_cdevs[1].led_cdev, acer_mcled_cdevs[1].led_cdev.brightness);
		acer_mcled_cdevs[2].led_cdev.brightness_set(&acer_mcled_cdevs[2].led_cdev, acer_mcled_cdevs[2].led_cdev.brightness);
	}
}
EXPORT_SYMBOL(acer_leds_restore_state_extern);

void acer_leds_notify_brightness_change_extern(void) {
	int status;
	u32 result;

	if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR) {
		status = acer_evaluate_method(ACER_CMD_GET_KB_WHITE_LEDS, 0, &result);
		pr_debug("Firmware set brightness: %u\n", result);
		acer_led_cdev.brightness = result;
		led_classdev_notify_brightness_hw_changed(&acer_led_cdev, result);
	}
}
EXPORT_SYMBOL(acer_leds_notify_brightness_change_extern);

// TODO Not used externaly, but only on init. Should not be exposed because it would require a correct
// led_classdev_notify_brightness_hw_changed implementation when used outside of init.
void acer_leds_set_brightness_extern(enum led_brightness brightness) {
	if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_FIXED_COLOR) {
		acer_led_cdev.brightness_set(&acer_led_cdev, brightness);
	}
	else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB) {
		acer_mcled_cdevs[0].led_cdev.brightness_set(&acer_mcled_cdevs[0].led_cdev, brightness);
	}
	else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB) {
		acer_mcled_cdevs[0].led_cdev.brightness_set(&acer_mcled_cdevs[0].led_cdev, brightness);
		acer_mcled_cdevs[1].led_cdev.brightness_set(&acer_mcled_cdevs[1].led_cdev, brightness);
		acer_mcled_cdevs[2].led_cdev.brightness_set(&acer_mcled_cdevs[2].led_cdev, brightness);
	}
}
EXPORT_SYMBOL(acer_leds_set_brightness_extern);

// TODO Not used externaly, but only on init. Should not be exposed because it would require a correct
// led_classdev_notify_brightness_hw_changed equivalent for color implementation when used outside of init.
void acer_leds_set_color_extern(u32 color) {
	if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_1_ZONE_RGB) {
		acer_mcled_cdevs[0].subled_info[0].intensity = (color >> 16) & 0xff;
		acer_mcled_cdevs[0].subled_info[1].intensity = (color >> 8) & 0xff;
		acer_mcled_cdevs[0].subled_info[2].intensity = color & 0xff;
		acer_mcled_cdevs[0].led_cdev.brightness_set(&acer_mcled_cdevs[0].led_cdev, acer_mcled_cdevs[0].led_cdev.brightness);
	}
	else if (acer_kb_backlight_type == ACER_KB_BACKLIGHT_TYPE_3_ZONE_RGB) {
		acer_mcled_cdevs[0].subled_info[0].intensity = (color >> 16) & 0xff;
		acer_mcled_cdevs[0].subled_info[1].intensity = (color >> 8) & 0xff;
		acer_mcled_cdevs[0].subled_info[2].intensity = color & 0xff;
		acer_mcled_cdevs[0].led_cdev.brightness_set(&acer_mcled_cdevs[0].led_cdev, acer_mcled_cdevs[0].led_cdev.brightness);
		acer_mcled_cdevs[1].subled_info[0].intensity = (color >> 16) & 0xff;
		acer_mcled_cdevs[1].subled_info[1].intensity = (color >> 8) & 0xff;
		acer_mcled_cdevs[1].subled_info[2].intensity = color & 0xff;
		acer_mcled_cdevs[1].led_cdev.brightness_set(&acer_mcled_cdevs[1].led_cdev, acer_mcled_cdevs[1].led_cdev.brightness);
		acer_mcled_cdevs[2].subled_info[0].intensity = (color >> 16) & 0xff;
		acer_mcled_cdevs[2].subled_info[1].intensity = (color >> 8) & 0xff;
		acer_mcled_cdevs[2].subled_info[2].intensity = color & 0xff;
		acer_mcled_cdevs[2].led_cdev.brightness_set(&acer_mcled_cdevs[2].led_cdev, acer_mcled_cdevs[2].led_cdev.brightness);
	}
}
EXPORT_SYMBOL(acer_leds_set_color_extern);

MODULE_LICENSE("GPL");

#endif // ACER_LEDS_H
