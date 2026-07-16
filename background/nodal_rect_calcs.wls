#!/usr/bin/env wolframscript
(* ::Package:: *)

Clear["Global`*"];

dui[f_]:=D[f,um]//FullSimplify;
dwi[f_]:=D[f,wm]//FullSimplify;
duj[f_]:=-D[f,um]//FullSimplify;
dwj[f_]:=-D[f,wm]//FullSimplify;

vmm2:= um^2+wm^2;
vmm := Sqrt[um^2+wm^2];

Print["KCL Y Real:"]
kclyr=gm um-bm wm

Print["KCL Y Imag:"]
kclyi=bm um+gm wm

Print["KCL Y Real Derivs:"]
{dui[kclyr], dwi[kclyr]}

Print["KCL Y Imag Derivs:"]
{dui[kclyi], dwi[kclyi]}

Print["KCL A Real:"]
kclar=(cm um+dm wm)/vmm

Print["KCL A Imag:"]
kclai=(-dm um+cm wm)/vmm

Print["KCL A Real Derivs:"]
{dui[kclar], dwi[kclar]}

Print["KCL A Imag Derivs:"]
{dui[kclai], dwi[kclai]}

Print["KCL S Real:"]
kclsr = (pm*um+qm*wm)/vmm2

Print["KCL S Imag:"]
kclsi = (pm*wm-qm*um)/vmm2

Print["KCL S Real Derivs:"]
{dui[kclsr], dwi[kclsr]}

Print["KCL S Imag Derivs:"]
{dui[kclsi], dwi[kclsi]}

Print["KCL SGN Real:"]
kclsgnr=(-pg ui-qg wi)/(ui^2+wi^2)

Print["KCL SGN Imag:"]
kclsgni=(qg ui-pg wi)/(ui^2+wi^2)

Print["KCL SGN Real Derivs:"]
{D[kclsgnr, ui]//FullSimplify, D[kclsgnr, wi]//FullSimplify}

Print["KCL SGN Imag Derivs:"]
{D[kclsgni, ui]//FullSimplify, D[kclsgni, wi]//FullSimplify}

Print["KCL SGB Real:"]
kclsgbr=(-pg um-qg wm)/(um^2+wm^2)

Print["KCL SGB Imag:"]
kclsgbi=(qg um-pg wm)/(um^2+wm^2)

Print["KCL SGB Real Derivs:"]
{dui[kclsgbr] // FullSimplify, dwi[kclsgbr]//FullSimplify}

Print["KCL SGB Imag Derivs:"]
{dui[kclsgbi]//FullSimplify, dwi[kclsgbi]//FullSimplify}



