+++
title = "Action Time!"
description = "More refactors and features with behavior trees on Bevy 0.17's event triggers, plus WebDriver BiDi, analytics and PDF exports."
tags = ["release-notes", "behavior-trees"]
created = "2025-10-07"
authors = ["Pete Hayman"]
video_url = "https://youtu.be/yI9tuBsrW1M"

[RouteMeta]
slug = "action-time"
+++

This one's a bit of a groundwork harvest, I have some exciting ideas for where to take beet next but to do it properly we need a firm base to launch from.

## Action Time

The Bevy `0.17` release introduces event triggers, which I used as an opportunity to dramatically clean up the `beet_flow` integration, leaning into a simple request-response format. `beet_flow` will play a critical role in the next pass at `beet_router`, which is gonna be a behavior tree router! Can't wait.

## WebDriver BiDi

`beet_net` now has full cross-platform support for websockets, and we have an initial integration of WebDriver BiDi, the new standard set to replace Chrome DevTools Protocol. This will allow testing in an actual browser instead of just the html output (which in fairness has served pretty well so far!).

## Analytics

If a user experiences an error in a forest and no developer is around to hear, did it happen? Yes, yes it did.
Analytics get a bad rap for creepy misuse but are an essential tool for any production application, even documentation sites can gain valuable insights from knowing how far users get into a walkthrough before closing the page.
This release has a first pass at sending analytics, click some counters on the home page and open the network tab to see your PII getting sold on the dark web BWAHAHA lol jks GDPR compliant, no user tracking, no annoying cookie banners.

Also partial initial support for DynamoDB which will likely be the place where this data is going to be stored. For now it lives locally in `target/analytics`.

## PDF Exports

Preparing for a conference I didn't want to copy all my beetmash stuff into a word doc and do formatting all over again so instead worked on print-to-pdf utilities like page breaks, hidden-unless-printing etc.

## Preparing for `bsn!`

It's been super handy getting to see Cart's progress with bsn, this harvest I've made some changes to align the templating system with the direction of bsn and the native bevy templating system. It's too early to say how much I'll be able to deprecate (100% would be a dream), but the closer I can get the model the more we'll be able to do.

That's about it for this harvest, all of these features put us in a steady position for having a lot of fun in the coming weeks.
