
const animations = [
  "Idle",
  "Walking_A",
  "Walking_B",
  "Walking_C",
  "Walking_Backwards",
  "Running_A",
  "Running_B",
  "Running_Strafe_Right",
  "Running_Strafe_Left",
  "Jump_Full_Short",
  "Jump_Full_Long",
  "Jump_Start",
  "Jump_Idle",
  "Jump_Land",
  "Dodge_Right",
  "Dodge_Left",
  "Dodge_Forward",
  "Dodge_Backward",
  "PickUp",
  "Use_Item",
  "Throw",
  "Interact",
  "Cheer",
  "Hit_A",
  "Hit_B",
  "Death_A",
  "Death_A_Pose",
  "Death_B",
  "Death_B_Pose",
  "1H_Melee_Attack_Chop",
  "1H_Melee_Attack_Slice_Diagonal",
  "1H_Melee_Attack_Slice_Horizontal",
  "1H_Melee_Attack_Stab",
  "2H_Melee_Idle",
  "2H_Melee_Attack_Chop",
  "2H_Melee_Attack_Slice",
  "2H_Melee_Attack_Stab",
  "2H_Melee_Attack_Spin",
  "2H_Melee_Attack_Spinning",
  "Dualwield_Melee_Attack_Chop",
  "Dualwield_Melee_Attack_Slice",
  "Dualwield_Melee_Attack_Stab",
  "Unarmed_Idle",
  "Unarmed_Pose",
  "Unarmed_Melee_Attack_Punch_A",
  "Unarmed_Melee_Attack_Punch_B",
  "Unarmed_Melee_Attack_Kick",
  "Block",
  "Blocking",
  "Block_Hit",
  "Block_Attack",
  "1H_Ranged_Aiming",
  "1H_Ranged_Shoot",
  "1H_Ranged_Shooting",
  "1H_Ranged_Reload",
  "2H_Ranged_Aiming",
  "2H_Ranged_Shoot",
  "2H_Ranged_Shooting",
  "2H_Ranged_Reload",
  "Spellcast_Shoot",
  "Spellcast_Raise",
  "Spellcast_Long",
  "Spellcast_Charge",
  "Lie_Down",
  "Lie_Idle",
  "Lie_Pose",
  "Lie_StandUp",
  "Sit_Chair_Down",
  "Sit_Chair_Idle",
  "Sit_Chair_Pose",
  "Sit_Chair_StandUp",
  "Sit_Floor_Down",
  "Sit_Floor_Idle",
  "Sit_Floor_Pose",
  "Sit_Floor_StandUp",
]  

type Gltf={
  animations: {
    name: string
  }[]
}

if (import.meta.main) {
  const buff = await Deno.readFile('../assets/kaykit-adventurers/Barbarian.gltf')
  const txt = new TextDecoder().decode(buff)
  const gltf:Gltf = JSON.parse(txt)
  // console.dir(gltf.animations)
  const indices = animations.reduce((acc, name) => {
    acc[name] = gltf.animations.findIndex(anim => 
      anim.name === name)
    return acc
  }, {} as any)
  console.dir(indices)

}
